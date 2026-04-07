<script lang="ts">
  interface TranscriptEntry {
    text: string;
    language: string;
    timestamp: number;
    is_final: boolean;
  }

  interface TranslationEntry {
    original: string;
    translated: string;
    source_lang: string;
    target_lang: string;
    timestamp: number;
  }

  let { sourceLanguage, targetLanguage, transcriptions, translations }: {
    sourceLanguage: string;
    targetLanguage: string;
    transcriptions: TranscriptEntry[];
    translations: TranslationEntry[];
  } = $props();

  function formatTime(ms: number): string {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  }
</script>

<div class="dual-panel">
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">Source ({sourceLanguage})</span>
      <span class="panel-title" style="font-size: 0.85rem; color: rgba(255,255,255,0.6);">
        {transcriptions.length} segments
      </span>
    </div>
    <div class="panel-content">
      {#each transcriptions as item}
        <div class="transcription-item" class:provisional={!item.is_final}>
          <div class="transcription-text">{item.text}</div>
          <div class="transcription-meta">
            {item.language} |
            {formatTime(item.timestamp)}
            {#if !item.is_final}
              <span class="provisional-tag">...</span>
            {/if}
          </div>
        </div>
      {/each}
      {#if transcriptions.length === 0}
        <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 2rem;">
          Waiting for speech input...
        </div>
      {/if}
    </div>
  </div>

  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">Translation ({targetLanguage})</span>
      <span class="panel-title" style="font-size: 0.85rem; color: rgba(255,255,255,0.6);">
        {translations.length} translations
      </span>
    </div>
    <div class="panel-content">
      {#each translations as item}
        <div class="transcription-item">
          <div class="transcription-text">{item.translated}</div>
          {#if item.original}
            <div class="original-text">{item.original}</div>
          {/if}
          <div class="transcription-meta">
            {item.source_lang} -> {item.target_lang} |
            {formatTime(item.timestamp)}
          </div>
        </div>
      {/each}
      {#if translations.length === 0}
        <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 2rem;">
          Translations will appear here...
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .dual-panel {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 1rem;
    height: 400px;
  }

  .panel {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .panel-title {
    font-size: 1rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.9);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .transcription-item {
    padding: 0.75rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    animation: fadeIn 0.3s ease-in;
  }

  .transcription-item.provisional {
    opacity: 0.7;
    border-left: 3px solid rgba(100, 108, 255, 0.3);
  }

  .transcription-text {
    font-size: 0.95rem;
    line-height: 1.4;
    margin-bottom: 0.25rem;
  }

  .original-text {
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.4);
    margin-bottom: 0.25rem;
    font-style: italic;
  }

  .transcription-meta {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.5);
  }

  .provisional-tag {
    color: rgba(100, 108, 255, 0.6);
    margin-left: 0.25rem;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 768px) {
    .dual-panel {
      grid-template-columns: 1fr;
      height: auto;
    }
  }
</style>
