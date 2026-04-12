/**
 * Soniox WebSocket Client for Auralis
 *
 * Connects to wss://stt-rt.soniox.com/transcribe-websocket for real-time
 * speech-to-text with live translation.
 *
 * Features:
 * - Auto-reconnect with exponential backoff (max 3 attempts)
 * - Seamless session reset every 3 minutes (make-before-break)
 * - Context carryover via translation history
 * - Connection keepalive every 15 seconds
 * - Token-by-token response parsing with translation_status routing
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TranslationType = "one_way" | "two_way";

export type ConnectionStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

export interface SonioxConfig {
  api_key: string;
  source_language: string;
  target_language: string;
  translation_type: TranslationType;
  endpoint_delay?: number;

  /** Called when finalized original-language text is received. */
  onOriginal: (text: string, is_final: boolean, language?: string) => void;
  /** Called when finalized translated text is received. */
  onTranslation: (text: string, is_final: boolean) => void;
  /** Called whenever the connection status changes. */
  onStatusChange: (status: ConnectionStatus) => void;
  /** Called on unrecoverable or noteworthy errors. */
  onError: (error: string) => void;
}

/** A single token returned by the Soniox API. */
interface SonioxToken {
  text: string;
  is_final: boolean;
  translation_status: "original" | "translation" | "none";
  speaker?: number;
  language?: string;
  confidence?: number;
}

/** The shape of every JSON message coming from Soniox. */
interface SonioxMessage {
  tokens?: SonioxToken[];
  error_code?: number;
  error_message?: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SONIOX_ENDPOINT = "wss://stt-rt.soniox.com/transcribe-websocket";

const MAX_RECONNECT_ATTEMPTS = 3;
const BASE_RECONNECT_DELAY_MS = 2000;

/** Full session duration before we perform a seamless reset. */
const SESSION_DURATION_MS = 3 * 60 * 1000;

/** How often to send a keepalive frame during silence. */
const KEEPALIVE_INTERVAL_MS = 15_000;

/** Maximum characters of recent translations retained for context carryover. */
const CONTEXT_HISTORY_CHARS = 500;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/**
 * A typed, zero-dependency WebSocket client for the Soniox real-time STT
 * service.
 *
 * Typical usage:
 * ```ts
 * const client = new SonioxClient({
 *   api_key: "...",
 *   source_language: "en",
 *   target_language: "vi",
 *   translation_type: "one_way",
 *   onOriginal:    (text, fin) => { ... },
 *   onTranslation: (text, fin) => { ... },
 *   onStatusChange: (s) => { ... },
 *   onError:        (e) => { ... },
 * });
 * client.connect();
 * // ... feed PCM data from microphone ...
 * client.sendAudio(pcmChunk);
 * // ... when done ...
 * client.disconnect();
 * ```
 */
export class SonioxClient {
  // ---- public config (immutable after construction) -----------------------
  private readonly config: SonioxConfig;

  // ---- WebSocket state ----------------------------------------------------
  private ws: WebSocket | null = null;

  /**
   * A secondary WebSocket created during a seamless session reset.
   * Once the new socket is open and configured the old one is retired.
   */
  private pendingWs: WebSocket | null = null;

  private isConnected = false;
  private intentionalDisconnect = false;
  private reconnectAttempts = 0;

  // ---- timers -------------------------------------------------------------
  private sessionTimer: ReturnType<typeof setTimeout> | null = null;
  private keepaliveTimer: ReturnType<typeof setInterval> | null = null;

  // ---- context carryover --------------------------------------------------
  private recentTranslations: string[] = [];

  // ---- internal bookkeeping for old sockets during make-before-break ------
  private retiredSockets = new WeakSet<WebSocket>();

  // =========================================================================
  // Lifecycle
  // =========================================================================

  constructor(config: SonioxConfig) {
    this.config = config;
  }

  // =========================================================================
  // Public API
  // =========================================================================

  /** Open a WebSocket connection and send the initial config message. */
  connect(): void {
    if (!this.config.api_key) {
      this.emitStatus("error");
      this.config.onError("API key is required.");
      return;
    }

    this.intentionalDisconnect = false;
    this.reconnectAttempts = 0;
    this.recentTranslations = [];

    this.doConnect(null);
  }

  /** Send raw PCM audio data (signed 16-bit little-endian, 16 kHz, mono). */
  sendAudio(pcmData: ArrayBuffer | Uint8Array): void {
    const socket = this.activeSocket();
    if (socket) {
      socket.send(pcmData);
    }
  }

  /** Gracefully close the connection. No reconnect will be attempted. */
  disconnect(): void {
    this.intentionalDisconnect = true;
    this.clearSessionTimer();
    this.clearKeepalive();

    const socket = this.activeSocket();

    if (socket) {
      try {
        if (socket.readyState === WebSocket.OPEN) {
          // Sending an empty buffer signals graceful end-of-stream to Soniox.
          socket.send(new ArrayBuffer(0));
        }
        socket.close(1000, "User disconnected");
      } catch {
        // Swallow -- we are tearing down regardless.
      }
    }

    // Also close any pending (mid-reset) socket.
    if (this.pendingWs) {
      try {
        this.pendingWs.close(1000, "User disconnected");
      } catch {
        // Swallow.
      }
      this.pendingWs = null;
    }

    this.ws = null;
    this.isConnected = false;
    this.emitStatus("disconnected");
  }

  // =========================================================================
  // Connection logic
  // =========================================================================

  /**
   * Core connect / reconnect routine.
   *
   * @param carryoverContext  Recent translation text to include as context
   *                          when performing a seamless session reset.
   */
  private doConnect(carryoverContext: string | null): void {
    this.emitStatus("connecting");

    let newWs: WebSocket;
    try {
      newWs = new WebSocket(SONIOX_ENDPOINT);
    } catch (err) {
      this.emitStatus("error");
      this.config.onError(
        `Failed to create WebSocket: ${err instanceof Error ? err.message : String(err)}`
      );
      return;
    }

    // If we are already connected, this new socket is a "pending" replacement
    // (make-before-break). Otherwise it is the primary socket.
    const isReset = this.ws !== null && this.ws.readyState === WebSocket.OPEN;

    if (isReset) {
      this.pendingWs = newWs;
    }

    newWs.onopen = () => {
      this.onSocketOpen(newWs, carryoverContext, isReset);
    };

    newWs.onmessage = (event: MessageEvent) => {
      if (this.retiredSockets.has(newWs)) return;
      this.onSocketMessage(event);
    };

    newWs.onerror = () => {
      if (this.retiredSockets.has(newWs)) return;
      this.config.onError("WebSocket error occurred");
    };

    newWs.onclose = (event: CloseEvent) => {
      this.onSocketClose(newWs, event);
    };
  }

  // ---- Socket lifecycle handlers ------------------------------------------

  private onSocketOpen(
    newWs: WebSocket,
    carryoverContext: string | null,
    isReset: boolean
  ): void {
    // Build and send config message.
    const configMsg = this.buildConfigMessage(carryoverContext);
    newWs.send(JSON.stringify(configMsg));

    if (isReset) {
      // Make-before-break: retire the old socket now that the new one is ready.
      const oldWs = this.ws;
      if (oldWs) {
        this.retireSocket(oldWs, "Session reset");
      }
    }

    // Promote new socket to primary.
    this.ws = newWs;
    this.pendingWs = null;
    this.isConnected = true;
    this.reconnectAttempts = 0;
    this.emitStatus("connected");

    // Start periodic session reset and keepalive.
    this.startSessionTimer();
    this.startKeepalive();
  }

  private onSocketMessage(event: MessageEvent): void {
    let data: SonioxMessage;
    try {
      data = JSON.parse(event.data as string) as SonioxMessage;
    } catch {
      return;
    }

    // API-level error?
    if (data.error_code !== undefined) {
      this.handleApiError(data);
      return;
    }

    this.handleTokens(data);
  }

  private onSocketClose(socket: WebSocket, event: CloseEvent): void {
    // Ignore close events from retired sockets.
    if (this.retiredSockets.has(socket)) {
      return;
    }

    // If a pending (mid-reset) socket closes, just clear it.
    if (socket === this.pendingWs) {
      this.pendingWs = null;
      return;
    }

    this.isConnected = false;

    if (this.ws === socket) {
      this.ws = null;
    }

    if (this.intentionalDisconnect) {
      this.emitStatus("disconnected");
      return;
    }

    // Map close codes to behaviour.
    switch (event.code) {
      case 1000:
        this.emitStatus("disconnected");
        break;

      case 1006: // Abnormal close -- no close frame received.
        this.tryReconnect("Connection lost unexpectedly");
        break;

      case 4001:
      case 4003:
        this.emitStatus("error");
        this.config.onError("Invalid API key. Please check your key.");
        break;

      case 4029:
        this.emitStatus("error");
        this.config.onError("Rate limit exceeded. Please wait and try again.");
        break;

      case 4002:
        this.emitStatus("error");
        this.config.onError(
          "Subscription issue. Please check your Soniox account."
        );
        break;

      default:
        this.tryReconnect(
          `Connection closed (code: ${event.code})`
        );
        break;
    }
  }

  // =========================================================================
  // Config message builder
  // =========================================================================

  private buildConfigMessage(
    carryoverContext: string | null
  ): Record<string, unknown> {
    const msg: Record<string, unknown> = {
      api_key: this.config.api_key,
      model: "stt-rt-v4",
      audio_format: "pcm_s16le",
      sample_rate: 16000,
      num_channels: 1,
      enable_endpoint_detection: true,
      max_endpoint_delay_ms: Math.round((this.config.endpoint_delay ?? 1.5) * 1000),
      enable_language_identification: this.config.translation_type === "two_way",
    };

    // Language hints.
    if (
      this.config.source_language &&
      this.config.source_language !== "auto"
    ) {
      msg.language_hints = [this.config.source_language];
    }

    // Translation configuration.
    if (this.config.translation_type === "two_way") {
      msg.translation = {
        type: "two_way",
        language_a: this.config.source_language,
        language_b: this.config.target_language,
      };
      msg.language_hints = [
        this.config.source_language,
        this.config.target_language,
      ];
    } else {
      msg.translation = {
        type: "one_way",
        target_language: this.config.target_language,
      };
    }

    // Context carryover from previous session.
    if (carryoverContext) {
      msg.context = {
        text: `Recent conversation: ${carryoverContext}`,
      };
    }

    return msg;
  }

  // =========================================================================
  // Token handling
  // =========================================================================

  private handleTokens(data: SonioxMessage): void {
    if (!data.tokens || data.tokens.length === 0) return;

    let originalText = "";
    let translationText = "";
    let provisionalOriginal = "";
    let hasEndpoint = false;
    let detectedLanguage: string | undefined;

    for (const token of data.tokens) {
      // Special endpoint marker.
      if (token.text === "<end>") {
        hasEndpoint = true;
        continue;
      }

      // Capture detected language from original tokens.
      if (token.language && token.translation_status !== "translation") {
        detectedLanguage = token.language;
      }

      switch (token.translation_status) {
        case "original":
          if (token.is_final) {
            originalText += token.text;
          } else {
            provisionalOriginal += token.text;
          }
          break;

        case "translation":
          if (token.is_final) {
            translationText += token.text;
          }
          break;

        case "none":
          // In two-way mode, third-language speech comes through as "none".
          // Surface it as untranslated original text.
          if (token.is_final) {
            originalText += token.text;
          } else {
            provisionalOriginal += token.text;
          }
          break;
      }
    }

    // Emit finalized original text with detected language.
    if (originalText.trim()) {
      this.config.onOriginal(originalText, true, detectedLanguage);
    }

    // Emit provisional original text with detected language.
    if (provisionalOriginal.trim()) {
      this.config.onOriginal(provisionalOriginal, false, detectedLanguage);
    }

    // Emit finalized translation text + store for carryover.
    if (translationText.trim()) {
      this.config.onTranslation(translationText, true);
      this.addTranslationToHistory(translationText);
    }

    // When an endpoint is detected with no remaining provisional text,
    // signal an empty provisional to let consumers clear their display.
    if (
      hasEndpoint &&
      !provisionalOriginal.trim() &&
      (originalText.trim() || translationText.trim())
    ) {
      this.config.onOriginal("", false);
    }
  }

  // =========================================================================
  // API error handling
  // =========================================================================

  private handleApiError(data: SonioxMessage): void {
    const code = data.error_code ?? 0;
    const message = data.error_message ?? "Unknown API error";

    // Timeout errors are transient -- attempt reconnect.
    if (code === 408) {
      this.tryReconnect("Request timeout");
      return;
    }

    let userMessage: string;
    switch (code) {
      case 401:
        userMessage = "Invalid API key. Please check your key.";
        break;
      case 429:
        userMessage = "Rate limit exceeded. Please wait a moment.";
        break;
      case 402:
        userMessage = "Insufficient credits. Check your Soniox account.";
        break;
      case 400:
        userMessage = `Config error: ${message}`;
        break;
      default:
        userMessage = message;
        break;
    }

    this.emitStatus("error");
    this.config.onError(userMessage);
  }

  // =========================================================================
  // Reconnection
  // =========================================================================

  private tryReconnect(reason: string): void {
    if (this.reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      this.emitStatus("error");
      this.config.onError(
        `${reason}. Reconnect failed after ${MAX_RECONNECT_ATTEMPTS} attempts.`
      );
      return;
    }

    this.reconnectAttempts += 1;

    // Exponential backoff: 2s, 4s, 6s.
    const delay = BASE_RECONNECT_DELAY_MS * this.reconnectAttempts;

    this.emitStatus("connecting");
    this.config.onError(
      `${reason}. Reconnecting (${this.reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})...`
    );

    setTimeout(() => {
      if (!this.intentionalDisconnect) {
        const carryover = this.getCarryoverContext();
        this.doConnect(carryover);
      }
    }, delay);
  }

  // =========================================================================
  // Session timer (seamless reset every 3 minutes)
  // =========================================================================

  private startSessionTimer(): void {
    this.clearSessionTimer();
    this.sessionTimer = setTimeout(() => {
      this.seamlessReset();
    }, SESSION_DURATION_MS);
  }

  private clearSessionTimer(): void {
    if (this.sessionTimer !== null) {
      clearTimeout(this.sessionTimer);
      this.sessionTimer = null;
    }
  }

  /**
   * Perform a make-before-break session reset.
   *
   * Opens a fresh WebSocket and, once it is ready, retires the old one.
   * Audio capture continues uninterrupted throughout.
   */
  private seamlessReset(): void {
    if (this.intentionalDisconnect) return;

    const carryover = this.getCarryoverContext();
    this.doConnect(carryover);
  }

  // =========================================================================
  // Keepalive
  // =========================================================================

  private startKeepalive(): void {
    this.clearKeepalive();
    this.keepaliveTimer = setInterval(() => {
      const socket = this.activeSocket();
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: "keepalive" }));
      }
    }, KEEPALIVE_INTERVAL_MS);
  }

  private clearKeepalive(): void {
    if (this.keepaliveTimer !== null) {
      clearInterval(this.keepaliveTimer);
      this.keepaliveTimer = null;
    }
  }

  // =========================================================================
  // Context carryover
  // =========================================================================

  private addTranslationToHistory(text: string): void {
    this.recentTranslations.push(text);

    // Trim oldest entries to stay under the character budget.
    let total = this.recentTranslations.reduce(
      (sum, t) => sum + t.length,
      0
    );
    while (total > CONTEXT_HISTORY_CHARS && this.recentTranslations.length > 1) {
      const removed = this.recentTranslations.shift()!;
      total -= removed.length;
    }
  }

  private getCarryoverContext(): string | null {
    if (this.recentTranslations.length === 0) return null;
    return this.recentTranslations.join(" ").trim();
  }

  // =========================================================================
  // Helpers
  // =========================================================================

  /**
   * Returns the currently active WebSocket (the primary one, not a pending
   * replacement that is still handshaking).
   */
  private activeSocket(): WebSocket | null {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return this.ws;
    }
    return null;
  }

  /**
   * Retire a socket that is being replaced during a make-before-break reset.
   * Marks it so that subsequent close/error events are silently ignored.
   */
  private retireSocket(socket: WebSocket, reason: string): void {
    this.retiredSockets.add(socket);
    try {
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(new ArrayBuffer(0)); // Graceful end-of-stream signal.
      }
      socket.close(1000, reason);
    } catch {
      // Best-effort; the socket is being discarded anyway.
    }
  }

  private emitStatus(status: ConnectionStatus): void {
    this.config.onStatusChange(status);
  }
}
