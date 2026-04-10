<script lang="ts">
  import ModelDownloader from './ModelDownloader.svelte';
  import type { OperatingMode, TranslationType, AudioSource } from '../types';
  import { getLangLabel } from '../js/lang';
  import { tts } from '../js/tts';

  let {
    mode = 'cloud',
    sonioxApiKey = '',
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
    ttsProvider = 'webspeech' as 'webspeech' | 'edge',
    onSave,
    onBack,
  }: {
    mode?: OperatingMode;
    sonioxApiKey?: string;
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
    ttsProvider?: 'webspeech' | 'edge';
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
      tts_provider: 'webspeech' | 'edge';
    }) => void;
    onBack: () => void;
  } = $props();

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
  let localTtsProvider: 'webspeech' | 'edge' = $state('webspeech');
  let voiceDropdownOpen = $state(false);
  let availableVoices: Array<{ name: string; lang: string; local: boolean; gender?: string }> = $state([]);
  let activeTab = $state<'translation' | 'display' | 'tts' | 'about'>('translation');

  // Update checker state
  let appVersion = $state('...');
  let updateStatus: 'idle' | 'checking' | 'up-to-date' | 'available' | 'error' = $state('idle');
  let latestVersion = $state('');
  let updateUrl = $state('');

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
  });

  // Load version when About tab is opened
  $effect(() => {
    if (activeTab === 'about') {
      loadVersion();
    }
  });

  // Load voices when TTS tab is opened or provider changes
  $effect(() => {
    if (activeTab === 'tts') {
      tts.setProvider(localTtsProvider);
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

  function handleSave() {
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
      const resp = await fetch('https://gitlab.com/api/v4/projects/auralis3%2Fauralis/releases?per_page=1');
      if (!resp.ok) throw new Error('not_public');
      const data = await resp.json();
      if (!Array.isArray(data) || data.length === 0) {
        updateStatus = 'up-to-date';
        return;
      }
      latestVersion = (data[0].tag_name as string).replace(/^v/, '');
      updateUrl = data[0]._links?.self ?? `https://gitlab.com/auralis3/auralis/-/releases`;
      if (latestVersion !== appVersion) {
        updateStatus = 'available';
      } else {
        updateStatus = 'up-to-date';
      }
    } catch {
      // Private repo or no releases yet — show up-to-date since there's nothing to update to
      updateStatus = 'up-to-date';
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

  function toggleVoiceDropdown() {
    voiceDropdownOpen = !voiceDropdownOpen;
  }

  function selectVoice(name: string) {
    localTtsVoice = name;
    voiceDropdownOpen = false;
  }

  function handleVoiceClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.voice-dropdown-container')) {
      voiceDropdownOpen = false;
    }
  }
</script>

<svelte:window onclick={handleVoiceClickOutside} />

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
  <div class="tab-content">
    {#if activeTab === 'translation'}
      <div class="settings-section">
        <!-- 1. Mode selector -->
        <div class="section-label">Translation Mode</div>
        <p class="section-desc">Choose how speech is recognized and translated.</p>
        <div class="mode-cards">
          <label class="mode-card" class:active={localMode === 'cloud'}>
            <input type="radio" name="mode" value="cloud" bind:group={localMode} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">Cloud (Soniox)</span>
              <span class="mode-desc">~150ms latency, requires internet</span>
            </div>
          </label>
          <label class="mode-card" class:active={localMode === 'offline'}>
            <input type="radio" name="mode" value="offline" bind:group={localMode} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">Offline (MLX)</span>
              <span class="mode-desc">~1s latency, works offline</span>
            </div>
          </label>
        </div>

        <!-- 2. Mode-specific setup (right after mode choice) -->
        {#if localMode === 'cloud'}
          <div class="field">
            <label for="api-key">Soniox API Key</label>
            <p class="field-desc">Required for cloud mode. Get a free key from soniox.com.</p>
            <input
              id="api-key"
              type="password"
              placeholder="Enter your Soniox API key"
              bind:value={localApiKey}
              disabled={isTranslating}
            />
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
          <label class="mode-card source-card" class:active={localAudioSource === 'system'}>
            <input type="radio" name="audio-source" value="system" bind:group={localAudioSource} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M2 10v4m4-6v8m4-10v12m4-8v4m4-6v8m4-4v0"/>
                </svg>
                System Audio
              </span>
              <span class="mode-desc">Capture computer audio</span>
            </div>
          </label>
          <label class="mode-card source-card" class:active={localAudioSource === 'both'}>
            <input type="radio" name="audio-source" value="both" bind:group={localAudioSource} disabled={isTranslating} />
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
              <span class="mode-desc">Mic + system audio</span>
            </div>
          </label>
        </div>

        <!-- Endpoint Delay card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Endpoint Delay</span>
            <span class="slider-card-value">{localEndpointDelay.toFixed(1)}s</span>
          </div>
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

        <!-- Provider cards -->
        <div class="section-label" style="margin-top: var(--space-lg);">Provider</div>
        <p class="section-desc">Choose between offline system voices or cloud-based Edge TTS.</p>
        <div class="mode-cards">
          <label class="mode-card" class:active={localTtsProvider === 'webspeech'}>
            <input type="radio" name="tts-provider" value="webspeech" bind:group={localTtsProvider} disabled={isTranslating || !localTtsEnabled} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                </svg>
                Web Speech
              </span>
              <span class="mode-desc">Offline, system voices</span>
            </div>
          </label>
          <label class="mode-card" class:active={localTtsProvider === 'edge'}>
            <input type="radio" name="tts-provider" value="edge" bind:group={localTtsProvider} disabled={isTranslating || !localTtsEnabled} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
                </svg>
                Edge TTS
              </span>
              <span class="mode-desc">Cloud, 40+ languages, free</span>
            </div>
          </label>
        </div>

        <!-- Voice selector -->
        <div class="section-label" style="margin-top: var(--space-lg);">Voice</div>
        {#if localTtsProvider === 'edge'}
          <p class="section-desc">Microsoft Neural voices for high-quality speech.</p>
          <div class="voice-dropdown-container">
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
              <div class="voice-dropdown">
                <button
                  class="voice-option"
                  class:active={localTtsVoice === ''}
                  onclick={() => selectVoice('')}
                >
                  <span class="voice-option-check">{localTtsVoice === '' ? '✓' : ''}</span>
                  <span class="voice-option-name">Auto</span>
                  <span class="voice-option-detail">Best for language</span>
                </button>
                {#each availableVoices as voice (voice.name)}
                  <button
                    class="voice-option"
                    class:active={localTtsVoice === voice.name}
                    onclick={() => selectVoice(voice.name)}
                  >
                    <span class="voice-option-check">{localTtsVoice === voice.name ? '✓' : ''}</span>
                    <span class="voice-option-name">{parseVoiceDisplay(voice).name}</span>
                    <span class="voice-option-detail">— {voice.gender ?? '—'}</span>
                    <span class="voice-option-flag">{getFlag(voice.lang ?? '', voice.name ?? '')}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <p class="section-desc">Web Speech uses your system's default voice. Switch to Edge TTS for voice selection.</p>
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
          <div class="about-desc">Real-time speech translation for macOS.</div>
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
                <a href={updateUrl} target="_blank" rel="noopener noreferrer" class="btn-primary" style="text-decoration: none; font-size: var(--font-size-xs); padding: 4px 14px; border-radius: 14px;">
                  Download
                </a>
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
            {#if updateStatus !== 'checking'}
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
  .mode-cards {
    display: flex;
    gap: var(--space-sm);
  }

  .mode-card {
    flex: 1;
    cursor: pointer;
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
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 320px;
    overflow-y: auto;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-solid);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    z-index: 50;
    padding: 4px;
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

  .voice-option-check {
    width: 14px;
    flex-shrink: 0;
    text-align: center;
    font-size: 12px;
    color: var(--accent);
    font-weight: 600;
  }

  .voice-option-name {
    font-weight: 500;
  }

  .voice-option-detail {
    color: var(--text-dim);
  }

  .voice-option-flag {
    margin-left: auto;
    font-size: 16px;
    flex-shrink: 0;
  }
</style>
