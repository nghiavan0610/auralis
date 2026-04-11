<script lang="ts">
  import ModelDownloader from './ModelDownloader.svelte';
  import type { OperatingMode, TranslationType, AudioSource } from '../types';
  import { getLangLabel } from '../js/lang';
  import { tts } from '../js/tts';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';

  interface PlatformInfo {
    os: string;
    system_audio_available: boolean;
    offline_mode_available: boolean;
  }

  let {
    mode = 'cloud',
    sonioxApiKey = '',
    googleApiKey = '',
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    translationType = 'one_way',
    audioSource = 'microphone',
    isTranslating = false,
    opacity = 0.88,
    fontSize = 14,
    maxLines = 100,
    endpointDelay = 1.0,
    ttsEnabled = false,
    ttsVoice = '',
    ttsRate = 1.0,
    ttsProvider = 'webspeech' as 'webspeech' | 'edge' | 'google' | 'elevenlabs',
    elevenlabsApiKey = '',
    platformInfo = null as PlatformInfo | null,
    onSave,
    onBack,
  }: {
    mode?: OperatingMode;
    sonioxApiKey?: string;
    googleApiKey?: string;
    elevenlabsApiKey?: string;
    sourceLanguage?: string;
    targetLanguage?: string;
    translationType?: TranslationType;
    audioSource?: AudioSource;
    isTranslating?: boolean;
    opacity?: number;
    fontSize?: number;
    maxLines?: number;
    endpointDelay?: number;
    ttsEnabled?: boolean;
    ttsVoice?: string;
    ttsRate?: number;
    ttsProvider?: 'webspeech' | 'edge' | 'google' | 'elevenlabs';
    platformInfo?: PlatformInfo | null;
    onSave: (settings: {
      mode: OperatingMode;
      soniox_api_key: string;
      source_language: string;
      target_language: string;
      translation_type: TranslationType;
      audio_source: AudioSource;
      opacity: number;
      font_size: number;
      max_lines: number;
      endpoint_delay: number;
      tts_enabled: boolean;
      tts_voice: string;
      tts_rate: number;
      tts_provider: 'webspeech' | 'edge' | 'google' | 'elevenlabs';
      google_api_key: string;
      elevenlabs_api_key: string;
    }) => void;
    onBack: () => void;
  } = $props();

  // Derived platform availability (default to true if info not yet loaded)
  let systemAudioAvailable = $derived(platformInfo?.system_audio_available ?? true);
  let offlineModeAvailable = $derived(platformInfo?.offline_mode_available ?? true);

  // Local copies for editing
  let localMode: OperatingMode = $state('cloud');
  let localApiKey = $state('');
  let localSource = $state('en');
  let localTarget = $state('vi');
  let localTranslationType: TranslationType = $state('one_way');
  let localAudioSource: AudioSource = $state('microphone');
  let localOpacity = $state(0.88);
  let localFontSize = $state(14);
  let localMaxLines = $state(100);
  let localEndpointDelay = $state(1.0);
  let localEndpointTenths = $state(10);
  let localTtsEnabled = $state(false);
  let localTtsVoice = $state('');
  let localTtsRate = $state(1.0);
  let localTtsRateTenths = $state(10);
  let localTtsProvider: 'webspeech' | 'edge' | 'google' | 'elevenlabs' = $state('webspeech');
  let localGoogleApiKey = $state('');
  let localElevenlabsApiKey = $state('');
  let voiceDropdownOpen = $state(false);
  let modeDropdownOpen = $state(false);
  let providerDropdownOpen = $state(false);
  let availableVoices: Array<{ name: string; lang: string; local: boolean; gender?: string }> = $state([]);
  let activeTab = $state<'translation' | 'display' | 'tts' | 'about'>('translation');

  // Update checker state
  let appVersion = $state('...');
  let updateStatus: 'idle' | 'checking' | 'up-to-date' | 'available' | 'downloading' | 'error' = $state('idle');
  let latestVersion = $state('');
  let updateProgress = $state(0);

  // Slider works with integer (30–100), opacity is 0.3–1.0
  let localOpacityPercent = $state(88);

  // Sync from props when they change (e.g. settings re-opened)
  $effect(() => {
    localMode = mode;
    localApiKey = sonioxApiKey;
    localSource = sourceLanguage;
    localTarget = targetLanguage;
    localTranslationType = translationType;
    localAudioSource = audioSource;
    localOpacity = opacity;
    localFontSize = fontSize;
    localMaxLines = maxLines;
    localOpacityPercent = Math.round(opacity * 100);
    localEndpointDelay = endpointDelay;
    localEndpointTenths = Math.round(endpointDelay * 10);
    localTtsEnabled = ttsEnabled;
    localTtsVoice = ttsVoice;
    localTtsRate = ttsRate;
    localTtsRateTenths = Math.round(ttsRate * 10);
    localTtsProvider = ttsProvider;
    localGoogleApiKey = googleApiKey;
    localElevenlabsApiKey = elevenlabsApiKey;
  });

  // Load version when About tab is opened
  $effect(() => {
    if (activeTab === 'about') {
      loadVersion();
    }
  });

  // Track previous provider to only reset voice on actual change
  let prevProvider: string = $state('');

  // Load voices when TTS tab is opened or provider changes
  $effect(() => {
    const provider = localTtsProvider; // Read to create reactive dependency
    if (activeTab === 'tts') {
      tts.setProvider(provider);
      // Only reset voice when provider actually changes (not on tab open)
      if (prevProvider && prevProvider !== provider) {
        localTtsVoice = '';
      }
      prevProvider = provider;
      tts.getVoices().then((v) => {
        // Sort: target language voices first, then others by lang
        const target = localTarget.toLowerCase();
        v.sort((a, b) => {
          const aMatch = a.lang.toLowerCase() === target ? 0 : 1;
          const bMatch = b.lang.toLowerCase() === target ? 0 : 1;
          if (aMatch !== bMatch) return aMatch - bMatch;
          return a.name.localeCompare(b.name);
        });
        availableVoices = v;
      });
    }
  });

  const languages = [
    { code: 'en', label: 'English' },
    { code: 'vi', label: 'Vietnamese' },
    { code: 'es', label: 'Spanish' },
    { code: 'fr', label: 'French' },
    { code: 'de', label: 'German' },
    { code: 'zh', label: 'Chinese' },
    { code: 'ja', label: 'Japanese' },
    { code: 'ko', label: 'Korean' },
    { code: 'pt', label: 'Portuguese' },
    { code: 'ru', label: 'Russian' },
    { code: 'ar', label: 'Arabic' },
    { code: 'hi', label: 'Hindi' },
  ];

  // Validation state
  let googleApiKeyError = $state(false);
  let elevenlabsApiKeyError = $state(false);
  let sonioxApiKeyError = $state(false);

  function handleSave() {
    // Validate: Cloud mode requires a Soniox API key
    if (localMode === 'cloud' && !localApiKey.trim()) {
      sonioxApiKeyError = true;
      activeTab = 'translation';
      setTimeout(() => {
        document.getElementById('api-key')?.focus();
      }, 50);
      return;
    }
    sonioxApiKeyError = false;

    // Validate: Google TTS requires an API key
    if (localTtsProvider === 'google' && !localGoogleApiKey.trim()) {
      googleApiKeyError = true;
      activeTab = 'tts';
      setTimeout(() => {
        document.getElementById('google-api-key')?.focus();
      }, 50);
      return;
    }
    googleApiKeyError = false;

    // Validate: ElevenLabs TTS requires an API key
    if (localTtsProvider === 'elevenlabs' && !localElevenlabsApiKey.trim()) {
      elevenlabsApiKeyError = true;
      activeTab = 'tts';
      setTimeout(() => {
        document.getElementById('elevenlabs-api-key')?.focus();
      }, 50);
      return;
    }
    elevenlabsApiKeyError = false;

    onSave({
      mode: localMode,
      soniox_api_key: localApiKey,
      source_language: localSource,
      target_language: localTarget,
      translation_type: localTranslationType,
      audio_source: localAudioSource,
      opacity: localOpacity,
      font_size: localFontSize,
      max_lines: localMaxLines,
      endpoint_delay: localEndpointDelay,
      tts_enabled: localTtsEnabled,
      tts_voice: localTtsVoice,
      tts_rate: localTtsRate,
      tts_provider: localTtsProvider,
      google_api_key: localGoogleApiKey,
      elevenlabs_api_key: localElevenlabsApiKey,
    });
  }

  async function loadVersion() {
    try {
      const { getVersion } = await import('@tauri-apps/api/app');
      appVersion = await getVersion();
    } catch {
      appVersion = '0.1.0';
    }
  }

  async function checkForUpdates() {
    updateStatus = 'checking';
    try {
      const update = await check();
      if (update?.available) {
        latestVersion = update.version;
        updateStatus = 'available';
      } else {
        updateStatus = 'up-to-date';
      }
    } catch {
      updateStatus = 'up-to-date';
    }
  }

  async function downloadAndInstallUpdate() {
    updateStatus = 'downloading';
    updateProgress = 0;
    try {
      const update = await check();
      if (!update?.available) {
        updateStatus = 'up-to-date';
        return;
      }
      let contentLength = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              updateProgress = Math.round((downloaded / contentLength) * 100);
            }
            break;
          case 'Finished':
            updateProgress = 100;
            break;
        }
      });
      await relaunch();
    } catch {
      updateStatus = 'error';
    }
  }

  // --- Voice display helpers ---

  const LANG_FLAGS: Record<string, string> = {
    en: '🇬🇧', vi: '🇻🇳', es: '🇪🇸', fr: '🇫🇷', de: '🇩🇪',
    zh: '🇨🇳', ja: '🇯🇵', ko: '🇰🇷', pt: '🇧🇷', ru: '🇷🇺',
    ar: '🇸🇦', hi: '🇮🇳',
    // Additional Edge TTS languages
    it: '🇮🇹', nl: '🇳🇱', pl: '🇵🇱', sv: '🇸🇪', da: '🇩🇰',
    fi: '🇫🇮', nb: '🇳🇴', tr: '🇹🇷', th: '🇹🇭', id: '🇮🇩',
    ms: '🇲🇾', tl: '🇵🇭', uk: '🇺🇦', cs: '🇨🇿', sk: '🇸🇰',
    hu: '🇭🇺', ro: '🇷🇴', bg: '🇧🇬', hr: '🇭🇷', sl: '🇸🇮',
    et: '🇪🇪', lv: '🇱🇻', lt: '🇱🇹', el: '🇬🇷', he: '🇮🇱',
    ca: '🇪🇸', eu: '🇪🇸', gl: '🇪🇸', mt: '🇲🇹', ga: '🇮🇪',
    cy: '🇬🇧', fil: '🇵🇭', te: '🇮🇳', ta: '🇮🇳', mr: '🇮🇳',
    gu: '🇮🇳', kn: '🇮🇳', ml: '🇮🇳', bn: '🇧🇩', ur: '🇵🇰',
    sw: '🇰🇪', am: '🇪🇹', jv: '🇮🇩', su: '🇮🇩', ne: '🇳🇵',
    km: '🇰🇭', lo: '🇱🇦', my: '🇲🇲', ka: '🇬🇪', az: '🇦🇿',
    uz: '🇺🇿', kk: '🇰🇿', mn: '🇲🇳', ps: '🇦🇫',
  };

  interface VoiceDisplay {
    value: string;
    name: string;
    gender: string;
    flag: string;
  }

  function getFlag(lang: string, name: string): string {
    try {
      const code = (lang ?? '').toLowerCase();
      if (LANG_FLAGS[code]) return LANG_FLAGS[code];
      const prefix = (name ?? '').split('-')[0]?.toLowerCase() ?? '';
      return LANG_FLAGS[prefix] ?? '🌐';
    } catch {
      return '🌐';
    }
  }

  function parseVoiceDisplay(voice: { name: string; lang: string; local: boolean; gender?: string }): VoiceDisplay {
    try {
      const flag = getFlag(voice.lang, voice.name);
      const gender = voice.gender ?? '—';
      let displayName = voice.name ?? '';
      const neuralMatch = displayName.match(/([A-Z][a-zA-Z]+)(?:Multilingual)?Neural$/);
      if (neuralMatch) {
        displayName = neuralMatch[1];
      }
      return { value: voice.name, name: displayName, gender, flag };
    } catch {
      return { value: voice.name ?? '', name: voice.name ?? '', gender: '—', flag: '🌐' };
    }
  }

  // Dropdown position for fixed positioning
  let dropdownPos = $state({ top: 0, left: 0, width: 0 });

  function positionDropdown(containerClass: string) {
    const trigger = document.querySelector(`${containerClass} .voice-trigger`) as HTMLElement;
    if (trigger) {
      const rect = trigger.getBoundingClientRect();
      dropdownPos = { top: rect.bottom + 4, left: rect.left, width: rect.width };
    }
  }

  function closeAllDropdowns() {
    modeDropdownOpen = false;
    providerDropdownOpen = false;
    voiceDropdownOpen = false;
  }

  function toggleModeDropdown() {
    const opening = !modeDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      modeDropdownOpen = true;
      positionDropdown('.mode-dropdown-container');
    }
  }

  function toggleProviderDropdown() {
    const opening = !providerDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      providerDropdownOpen = true;
      positionDropdown('.provider-dropdown-container');
    }
  }

  function toggleVoiceDropdown() {
    const opening = !voiceDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      voiceDropdownOpen = true;
      positionDropdown('.voice-dropdown-container');
    }
  }

  function selectVoice(name: string) {
    localTtsVoice = name;
    voiceDropdownOpen = false;
  }

  function handleGlobalClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.voice-dropdown-container') && !target.closest('.mode-dropdown-container') && !target.closest('.provider-dropdown-container')) {
      closeAllDropdowns();
    }
  }

  function handleScroll() {
    closeAllDropdowns();
  }

  let tabContentEl: HTMLElement | undefined = $state();
</script>

<svelte:window onclick={handleGlobalClick} />

<div class="settings-view">
  <!-- Header (in drag region) -->
  <div class="settings-header" data-tauri-drag-region>
    <button class="btn-icon" onclick={onBack} title="Back">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
    </button>
    <span class="settings-title" data-tauri-drag-region>Settings</span>
    <div style="flex:1" data-tauri-drag-region></div>
    <button class="btn-primary" onclick={handleSave} disabled={isTranslating}>Save</button>
  </div>

  <!-- Tabs -->
  <div class="tabs">
    <button class="tab" class:active={activeTab === 'translation'} onclick={() => activeTab = 'translation'}>
      Translation
    </button>
    <button class="tab" class:active={activeTab === 'display'} onclick={() => activeTab = 'display'}>
      Display
    </button>
    <button class="tab" class:active={activeTab === 'tts'} onclick={() => activeTab = 'tts'}>
      TTS
    </button>
    <button class="tab" class:active={activeTab === 'about'} onclick={() => activeTab = 'about'}>
      About
    </button>
  </div>

  <!-- Tab content -->
  <div class="tab-content" bind:this={tabContentEl} onscroll={handleScroll}>
    {#if activeTab === 'translation'}
      <div class="settings-section">
        <!-- 1. Mode selector -->
        <div class="section-label">Translation Mode</div>
        <p class="section-desc">Choose how speech is recognized and translated.</p>
        <div class="voice-dropdown-container mode-dropdown-container" class:open={modeDropdownOpen}>
          <button
            class="voice-trigger"
            onclick={toggleModeDropdown}
            disabled={isTranslating}
          >
            <span class="voice-trigger-text">
              {localMode === 'cloud' ? 'Cloud (Soniox)' : 'Offline (MLX)'}
            </span>
            <svg class="voice-trigger-arrow" class:open={modeDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
              <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>

          {#if modeDropdownOpen && !isTranslating}
            <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
              <button
                class="voice-option"
                class:active={localMode === 'cloud'}
                onclick={() => { localMode = 'cloud'; modeDropdownOpen = false; }}
              >
                <span class="voice-option-check">{localMode === 'cloud' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Cloud (Soniox)</span>
                  <span class="voice-option-desc">~150ms latency, requires internet</span>
                </div>
              </button>
              <button
                class="voice-option"
                class:active={localMode === 'offline'}
                class:disabled={!offlineModeAvailable}
                onclick={() => { if (offlineModeAvailable) { localMode = 'offline'; modeDropdownOpen = false; } }}
                disabled={!offlineModeAvailable}
              >
                <span class="voice-option-check">{localMode === 'offline' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Offline (MLX)</span>
                  <span class="voice-option-desc">{offlineModeAvailable ? '~3s latency, works offline' : 'macOS only'}</span>
                </div>
              </button>
            </div>
          {/if}
        </div>

        <!-- 2. Mode-specific setup (right after mode choice) -->
        {#if localMode === 'cloud'}
          <div class="field">
            <div class="label-row">
              <label for="api-key">Soniox API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for cloud mode. Get a free key from soniox.com.</p>
            <input
              id="api-key"
              type="password"
              class:input-error={sonioxApiKeyError}
              placeholder="Enter your Soniox API key"
              bind:value={localApiKey}
              oninput={() => sonioxApiKeyError = false}
              disabled={isTranslating}
            />
            {#if sonioxApiKeyError}
              <p class="field-error">API key is required for cloud mode.</p>
            {/if}
            <a href="https://soniox.com/api-keys" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {:else}
          <div class="field">
            <ModelDownloader />
          </div>
        {/if}

        <!-- 3. Languages -->
        <div class="section-label">Languages</div>
        <p class="section-desc">Set the source language (what you speak) and target language (what to translate into).</p>
        <div class="language-row">
          <div class="field">
            <label for="src-lang">Source</label>
            <select id="src-lang" bind:value={localSource} disabled={isTranslating}>
              {#each languages as lang}
                <option value={lang.code}>{lang.label}</option>
              {/each}
            </select>
          </div>

          <div class="lang-arrow">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M5 12h14M12 5l7 7-7 7"/>
            </svg>
          </div>

          <div class="field">
            <label for="tgt-lang">Target</label>
            <select id="tgt-lang" bind:value={localTarget} disabled={isTranslating}>
              {#each languages as lang}
                <option value={lang.code}>{lang.label}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- 4. Translation direction -->
        <div class="section-label">Direction</div>
        <p class="section-desc">One way translates source to target. Two way auto-detects which language is spoken and translates to the other.</p>
        <div class="direction-cards">
          <label class="mode-card" class:active={localTranslationType === 'one_way'}>
            <input type="radio" name="direction" value="one_way" bind:group={localTranslationType} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M5 12h14"/>
                </svg>
                One Way
              </span>
              <span class="mode-desc">{getLangLabel(localSource)} → {getLangLabel(localTarget)}</span>
            </div>
          </label>
          <label class="mode-card" class:active={localTranslationType === 'two_way'}>
            <input type="radio" name="direction" value="two_way" bind:group={localTranslationType} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M7 16l-4-4 4-4"/>
                  <path d="M17 8l4 4-4 4"/>
                  <line x1="3" y1="12" x2="21" y2="12"/>
                </svg>
                Two Way
              </span>
              <span class="mode-desc">{getLangLabel(localSource)} ↔ {getLangLabel(localTarget)}</span>
            </div>
          </label>
        </div>

        <!-- 5. Audio source -->
        <div class="section-label">Audio Source</div>
        <p class="section-desc">Where to capture audio from. System audio captures sound playing on your computer (requires Screen Recording permission).</p>
        <div class="source-cards">
          <label class="mode-card source-card" class:active={localAudioSource === 'microphone'}>
            <input type="radio" name="audio-source" value="microphone" bind:group={localAudioSource} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
                Microphone
              </span>
              <span class="mode-desc">Capture from your mic</span>
            </div>
          </label>
          <label class="mode-card source-card" class:active={localAudioSource === 'system'} class:disabled={!systemAudioAvailable}>
            <input type="radio" name="audio-source" value="system" bind:group={localAudioSource} disabled={isTranslating || !systemAudioAvailable} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M2 10v4m4-6v8m4-10v12m4-8v4m4-6v8m4-4v0"/>
                </svg>
                System Audio
              </span>
              <span class="mode-desc">{systemAudioAvailable ? 'Capture computer audio' : 'macOS only'}</span>
            </div>
          </label>
          <label class="mode-card source-card" class:active={localAudioSource === 'both'} class:disabled={!systemAudioAvailable}>
            <input type="radio" name="audio-source" value="both" bind:group={localAudioSource} disabled={isTranslating || !systemAudioAvailable} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
                Both
              </span>
              <span class="mode-desc">{systemAudioAvailable ? 'Mic + system audio' : 'macOS only'}</span>
            </div>
          </label>
        </div>

        <!-- Endpoint Delay card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Endpoint Delay</span>
            <span class="slider-card-value">{localEndpointDelay.toFixed(1)}s</span>
          </div>
          <p class="slider-card-desc">How long to wait after silence before finalizing a transcript segment. Lower for faster output, higher to avoid cutting off pauses.</p>
          <input
            type="range"
            min="5"
            max="30"
            step="1"
            bind:value={localEndpointTenths}
            oninput={() => localEndpointDelay = localEndpointTenths / 10}
            class="slider"
            style="--fill: {((localEndpointTenths - 5) / 25) * 100}%"
          />
        </div>
      </div>

    {:else if activeTab === 'display'}
      <div class="settings-section">
        <div class="section-label">Appearance</div>
        <p class="section-desc">Adjust how the overlay looks and feels.</p>

        <!-- Opacity card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Background Opacity</span>
            <span class="slider-card-value">{Math.round(localOpacity * 100)}%</span>
          </div>
          <input
            type="range"
            min="30"
            max="100"
            step="1"
            bind:value={localOpacityPercent}
            oninput={() => localOpacity = Number(localOpacityPercent) / 100}
            class="slider"
            style="--fill: {((localOpacityPercent - 30) / 70) * 100}%"
          />
        </div>

        <!-- Font Size card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Font Size</span>
            <span class="slider-card-value">{localFontSize}px</span>
          </div>
          <input
            type="range"
            min="12"
            max="24"
            step="1"
            bind:value={localFontSize}
            class="slider"
            style="--fill: {((localFontSize - 12) / 12) * 100}%"
          />
        </div>

        <!-- Max Lines card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Max Transcript Lines</span>
            <span class="slider-card-value">{localMaxLines}</span>
          </div>
          <input
            type="range"
            min="10"
            max="200"
            step="10"
            bind:value={localMaxLines}
            class="slider"
            style="--fill: {((localMaxLines - 10) / 190) * 100}%"
          />
        </div>
      </div>

    {:else if activeTab === 'tts'}
      <div class="settings-section">
        <!-- Enable toggle -->
        <div class="section-label">Text-to-Speech</div>
        <p class="section-desc">When enabled, translated text is automatically spoken aloud.</p>
        <div class="toggle-card">
          <div class="toggle-card-info">
            <span class="toggle-card-label">Speak translations aloud</span>
          </div>
          <button
            class="toggle"
            class:active={localTtsEnabled}
            onclick={() => localTtsEnabled = !localTtsEnabled}
            disabled={isTranslating}
            aria-label="Toggle text-to-speech"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>

        <!-- Provider dropdown -->
        <div class="section-label" style="margin-top: var(--space-lg);">Provider</div>
        <p class="section-desc">Choose between offline system voices or cloud-based TTS providers.</p>
        <div class="voice-dropdown-container provider-dropdown-container" class:open={providerDropdownOpen}>
          <button
            class="voice-trigger"
            onclick={toggleProviderDropdown}
            disabled={isTranslating || !localTtsEnabled}
          >
            <span class="voice-trigger-text">
              {#if localTtsProvider === 'webspeech'}
                Web Speech
              {:else if localTtsProvider === 'edge'}
                Edge TTS
              {:else if localTtsProvider === 'google'}
                Google Cloud TTS
              {:else}
                ElevenLabs
              {/if}
            </span>
            <svg class="voice-trigger-arrow" class:open={providerDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
              <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>

          {#if providerDropdownOpen && !isTranslating && localTtsEnabled}
            <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
              <button
                class="voice-option"
                class:active={localTtsProvider === 'webspeech'}
                onclick={() => { localTtsProvider = 'webspeech'; providerDropdownOpen = false; }}
              >
                <span class="voice-option-check">{localTtsProvider === 'webspeech' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Web Speech</span>
                  <span class="voice-option-desc">Offline, system voices</span>
                </div>
              </button>
              <button
                class="voice-option"
                class:active={localTtsProvider === 'edge'}
                onclick={() => { localTtsProvider = 'edge'; providerDropdownOpen = false; }}
              >
                <span class="voice-option-check">{localTtsProvider === 'edge' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Edge TTS</span>
                  <span class="voice-option-desc">Cloud, 40+ languages, free</span>
                </div>
              </button>
              <button
                class="voice-option"
                class:active={localTtsProvider === 'google'}
                onclick={() => { localTtsProvider = 'google'; providerDropdownOpen = false; }}
              >
                <span class="voice-option-check">{localTtsProvider === 'google' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Google Cloud TTS</span>
                  <span class="voice-option-desc">WaveNet voices, requires API key</span>
                </div>
              </button>
              <button
                class="voice-option"
                class:active={localTtsProvider === 'elevenlabs'}
                onclick={() => { localTtsProvider = 'elevenlabs'; providerDropdownOpen = false; }}
              >
                <span class="voice-option-check">{localTtsProvider === 'elevenlabs' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">ElevenLabs</span>
                  <span class="voice-option-desc">Ultra-realistic AI voices, API key</span>
                </div>
              </button>
            </div>
          {/if}
        </div>

        <!-- Google API Key (shown when Google is selected) -->
        {#if localTtsProvider === 'google'}
          <div class="field">
            <div class="label-row">
              <label for="google-api-key">Google Cloud API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for Google Cloud Text-to-Speech.</p>
            <input
              id="google-api-key"
              type="password"
              class:input-error={googleApiKeyError}
              placeholder="Enter your Google Cloud TTS API key"
              bind:value={localGoogleApiKey}
              oninput={() => googleApiKeyError = false}
              disabled={isTranslating}
            />
            {#if googleApiKeyError}
              <p class="field-error">API key is required to use Google Cloud TTS.</p>
            {/if}
            <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {/if}

        <!-- ElevenLabs API Key (shown when ElevenLabs is selected) -->
        {#if localTtsProvider === 'elevenlabs'}
          <div class="field">
            <div class="label-row">
              <label for="elevenlabs-api-key">ElevenLabs API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for ElevenLabs Text-to-Speech.</p>
            <input
              id="elevenlabs-api-key"
              type="password"
              class:input-error={elevenlabsApiKeyError}
              placeholder="Enter your ElevenLabs API key"
              bind:value={localElevenlabsApiKey}
              oninput={() => elevenlabsApiKeyError = false}
              disabled={isTranslating}
            />
            {#if elevenlabsApiKeyError}
              <p class="field-error">API key is required to use ElevenLabs TTS.</p>
            {/if}
            <a href="https://elevenlabs.io/app/settings/api-keys" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {/if}

        <!-- Voice selector -->
        <div class="section-label" style="margin-top: var(--space-lg);">Voice</div>
        {#if localTtsProvider === 'edge' || localTtsProvider === 'google' || localTtsProvider === 'elevenlabs'}
          <p class="section-desc">{localTtsProvider === 'edge' ? 'Microsoft Neural voices for high-quality speech.' : localTtsProvider === 'google' ? 'Google WaveNet/Neural2 voices for natural speech.' : 'ElevenLabs ultra-realistic AI voices.'}</p>
          <div class="voice-dropdown-container" class:open={voiceDropdownOpen}>
            <!-- Trigger -->
            <button
              class="voice-trigger"
              onclick={toggleVoiceDropdown}
              disabled={isTranslating || !localTtsEnabled}
            >
              {#if localTtsVoice}
                {@const selected = availableVoices.find(v => v.name === localTtsVoice)}
                {#if selected}
                  {@const display = parseVoiceDisplay(selected)}
                  <span class="voice-trigger-text">
                    <span class="voice-trigger-flag">{display.flag}</span>
                    {display.name} — {display.gender}
                  </span>
                {:else}
                  <span class="voice-trigger-text">{localTtsVoice}</span>
                {/if}
              {:else}
                <span class="voice-trigger-text voice-trigger-placeholder">Auto (best for language)</span>
              {/if}
              <svg class="voice-trigger-arrow" class:open={voiceDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
                <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            <!-- Dropdown list -->
            {#if voiceDropdownOpen && !isTranslating && localTtsEnabled}
              <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
                <button
                  class="voice-option"
                  class:active={localTtsVoice === ''}
                  onclick={() => selectVoice('')}
                >
                  <span class="voice-option-check">{localTtsVoice === '' ? '✓' : ''}</span>
                  <div class="voice-option-content">
                    <span class="voice-option-name">Auto</span>
                    <span class="voice-option-desc">Best for language</span>
                  </div>
                </button>
                {#each availableVoices as voice (voice.name)}
                  <button
                    class="voice-option"
                    class:active={localTtsVoice === voice.name}
                    onclick={() => selectVoice(voice.name)}
                  >
                    <span class="voice-option-check">{localTtsVoice === voice.name ? '✓' : ''}</span>
                    <div class="voice-option-content">
                      <span class="voice-option-name">{parseVoiceDisplay(voice).name}</span>
                      <span class="voice-option-desc">{voice.gender ?? '—'}</span>
                    </div>
                    <span class="voice-option-flag">{getFlag(voice.lang ?? '', voice.name ?? '')}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <p class="section-desc">Web Speech uses your system's default voice. Switch to another provider for voice selection.</p>
          <div class="voice-dropdown-container">
            <button class="voice-trigger" disabled>
              <span class="voice-trigger-text voice-trigger-placeholder">System default voice</span>
              <svg class="voice-trigger-arrow" width="10" height="6" viewBox="0 0 10 6" fill="none">
                <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
        {/if}

        <!-- Speed slider -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Speed</span>
            <span class="slider-card-value">{localTtsRate.toFixed(1)}x</span>
          </div>
          <p class="slider-card-desc">Playback speed for spoken translations. 1.0x is normal speed.</p>
          <input
            type="range"
            min="5"
            max="20"
            step="1"
            bind:value={localTtsRateTenths}
            oninput={() => localTtsRate = localTtsRateTenths / 10}
            class="slider"
            style="--fill: {((localTtsRateTenths - 5) / 15) * 100}%"
          />
        </div>
      </div>

    {:else}
      <div class="settings-section">
        <div class="about-card">
          <div class="about-logo">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
              <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
              <line x1="12" y1="19" x2="12" y2="23"/>
              <line x1="8" y1="23" x2="16" y2="23"/>
            </svg>
          </div>
          <div class="about-name">Auralis</div>
          <div class="about-version">v{appVersion}</div>
          <div class="about-desc">Real-time speech translation.</div>
        </div>

        <!-- Update section -->
        <div class="update-section">
          <div class="section-label">Updates</div>
          <div class="update-card">
            <div class="update-info">
              {#if updateStatus === 'idle'}
                <div class="update-status-icon">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"/>
                    <polyline points="12 6 12 12 16 14"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label">Last checked: never</span>
                </div>
              {:else if updateStatus === 'checking'}
                <div class="update-spinner"></div>
                <div class="update-text">
                  <span class="update-label">Checking for updates...</span>
                </div>
              {:else if updateStatus === 'up-to-date'}
                <div class="update-status-icon">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                    <polyline points="22 4 12 14.01 9 11.01"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--success);">You're up to date</span>
                  <span class="update-sub">v{appVersion} is the latest version</span>
                </div>
              {:else if updateStatus === 'available'}
                <div class="update-status-icon">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--accent);">Update available</span>
                  <span class="update-sub">v{latestVersion} is available</span>
                </div>
                <button onclick={downloadAndInstallUpdate} class="btn-primary" style="font-size: var(--font-size-xs); padding: 4px 14px; border-radius: 14px;">
                  Update
                </button>
              {:else if updateStatus === 'downloading'}
                <div class="update-spinner"></div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--accent);">Downloading update...</span>
                  <span class="update-sub">{updateProgress}%</span>
                </div>
              {:else}
                <div class="update-status-icon">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"/>
                    <line x1="15" y1="9" x2="9" y2="15"/>
                    <line x1="9" y1="9" x2="15" y2="15"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--danger);">Check failed</span>
                  <span class="update-sub">Could not reach update server</span>
                </div>
              {/if}
            </div>
            {#if updateStatus !== 'checking' && updateStatus !== 'downloading'}
              <button class="btn-icon update-refresh" onclick={checkForUpdates} title="Check for updates">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10"/>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <div class="about-footer">
          <span>Made with care by Sentia Lab</span>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    animation: fadeIn 0.15s ease;
  }

  /* Header */
  .settings-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    height: 42px;
    padding: 0 var(--space-sm);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .settings-title {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Tabs — pill style */
  .tabs {
    display: flex;
    gap: 4px;
    padding: var(--space-sm) var(--space-lg);
    flex-shrink: 0;
  }

  .tab {
    padding: 6px 14px;
    background: transparent;
    border: none;
    border-radius: 20px;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tab:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .tab.active {
    color: white;
    background: var(--accent);
  }

  /* Tab content */
  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-md) var(--space-lg) var(--space-lg);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .section-label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .section-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    margin-top: -6px;
    line-height: 1.5;
  }

  /* Mode cards — rounder */
  .mode-card {
    flex: 1;
    cursor: pointer;
  }

  .mode-card.disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .mode-card.disabled .mode-card-content {
    border-color: var(--border);
    background: transparent;
  }

  .mode-card input[type="radio"] {
    display: none;
  }

  .mode-card-content {
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: 2px;
    transition: all 0.2s ease;
  }

  .mode-card:hover .mode-card-content {
    border-color: var(--border-hover);
    background: var(--bg-secondary);
  }

  .mode-card.active .mode-card-content {
    border-color: var(--accent);
    background: var(--accent-dim);
  }

  .mode-name {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .mode-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .direction-cards {
    display: flex;
    gap: var(--space-sm);
  }

  .source-cards {
    display: flex;
    gap: var(--space-sm);
  }

  .source-card {
    flex: 1;
    min-width: 0;
  }

  /* Fields — rounder inputs */
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .field-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    line-height: 1.4;
  }

  .field-error {
    font-size: var(--font-size-xs);
    color: var(--danger);
    margin: 0;
    line-height: 1.4;
    font-weight: 500;
  }

  .label-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .badge-required {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #1a1b2e;
    background: var(--warning);
    padding: 1px 6px;
    border-radius: 4px;
    line-height: 1.5;
  }

  .input-error {
    border-color: var(--danger) !important;
    box-shadow: 0 0 0 3px rgba(255, 77, 77, 0.12) !important;
    animation: shake 0.3s ease;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    20%, 60% { transform: translateX(-4px); }
    40%, 80% { transform: translateX(4px); }
  }

  .field-link {
    font-size: var(--font-size-xs);
    align-self: flex-start;
  }

  /* Language row */
  .language-row {
    display: flex;
    align-items: flex-end;
    gap: var(--space-sm);
  }

  .lang-arrow {
    color: var(--text-dim);
    padding-bottom: var(--space-xs);
  }

  /* About */
  .about-card {
    text-align: center;
    padding: var(--space-xl) 0 var(--space-md);
  }

  .about-logo {
    width: 64px;
    height: 64px;
    border-radius: 18px;
    background: var(--accent-dim);
    border: 1px solid rgba(99, 140, 255, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto var(--space-md);
  }

  .about-name {
    font-size: var(--font-size-xl);
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .about-version {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin-bottom: var(--space-sm);
  }

  .about-desc {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
  }

  .about-footer {
    text-align: center;
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    padding-top: var(--space-lg);
  }

  /* Update section */
  .update-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .update-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-secondary);
  }

  .update-info {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .update-status-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--bg-primary);
  }

  .update-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .update-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .update-sub {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .update-refresh {
    color: var(--text-dim);
  }

  .update-refresh:hover {
    color: var(--text-primary);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .update-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
    margin: 0 4px;
  }

  /* Slider cards */
  .slider-card {
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: border-color 0.2s ease;
  }

  .slider-card:hover {
    border-color: var(--border-hover);
  }

  .slider-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .slider-card-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .slider-card-value {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    min-width: 40px;
    text-align: right;
  }

  .slider-card-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    line-height: 1.4;
  }

  /* Slider track */
  .slider {
    width: 100%;
    -webkit-appearance: none;
    appearance: none;
    height: 4px;
    border-radius: 2px;
    outline: none;
    cursor: pointer;
    background: linear-gradient(to right, var(--accent) 0%, var(--accent) var(--fill, 50%), rgba(255,255,255,0.08) var(--fill, 50%), rgba(255,255,255,0.08) 100%);
    transition: height 0.15s ease;
    border: none;
  }

  .slider:hover {
    height: 5px;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    border: none;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    transition: box-shadow 0.2s ease, transform 0.2s ease;
  }

  .slider::-webkit-slider-thumb:hover {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4), 0 0 0 5px rgba(99, 140, 255, 0.15);
    transform: scale(1.05);
  }

  /* Toggle switch */
  .toggle-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-secondary);
    transition: border-color 0.2s ease;
  }

  .toggle-card:hover {
    border-color: var(--border-hover);
  }

  .toggle-card-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .toggle {
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: rgba(255, 255, 255, 0.1);
    position: relative;
    cursor: pointer;
    transition: background 0.2s ease;
    flex-shrink: 0;
  }

  .toggle.active {
    background: var(--accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s ease;
    pointer-events: none;
  }

  .toggle.active .toggle-thumb {
    transform: translateX(20px);
  }

  .toggle:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* Voice dropdown */
  .voice-dropdown-container {
    position: relative;
    z-index: 50;
  }

  .voice-dropdown-container.open {
    z-index: 100;
  }

  .voice-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-solid);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    outline: none;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .voice-trigger:hover {
    border-color: var(--border-hover);
  }

  .voice-trigger:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 140, 255, 0.1);
  }

  .voice-trigger:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .voice-trigger-text {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .voice-trigger-flag {
    font-size: 16px;
  }

  .voice-trigger-placeholder {
    color: var(--text-dim);
  }

  .voice-trigger-arrow {
    flex-shrink: 0;
    color: var(--text-dim);
    transition: transform 0.2s ease;
  }

  .voice-trigger-arrow.open {
    transform: rotate(180deg);
  }

  .voice-dropdown {
    position: fixed;
    max-height: 280px;
    overflow-y: auto;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-solid);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    padding: 4px;
    z-index: 100;
    animation: dropdownSlideIn 0.12s ease;
  }

  @keyframes dropdownSlideIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .voice-option {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }

  .voice-option:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .voice-option.active {
    background: var(--accent-dim);
  }

  .voice-option.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .voice-option.disabled:hover {
    background: transparent;
  }

  .voice-option-check {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--accent);
    font-weight: 600;
  }

  .voice-option-content {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .voice-option-name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .voice-option-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.3;
  }

  .voice-option-flag {
    margin-left: auto;
    font-size: 16px;
    flex-shrink: 0;
  }
</style>
