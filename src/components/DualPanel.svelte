<script lang="ts">
  import type { STTSegment, Translation } from '@tauri-apps/api';

  export let sourceLanguage: string;
  export let targetLanguage: string;
  export let transcriptions: Array<{
    segment: STTSegment;
    language: string;
  }>;
  export let translations: Array<{
    translation: Translation;
  }>;

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
      <span class="panel-title">English ({sourceLanguage})</span>
      <span class="panel-title" style="font-size: 0.85rem; color: rgba(255,255,255,0.6);">
        {transcriptions.length} segments
      </span>
    </div>
    <div class="panel-content">
      {#each transcriptions as item, index}
        <div class="transcription-item">
          <div class="transcription-text">{item.segment.text}</div>
          <div class="transcription-meta">
            Confidence: {(item.segment.confidence * 100).toFixed(0)}% |
            {formatTime(item.segment.start)} - {formatTime(item.segment.end)}
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
      <span class="panel-title">Vietnamese ({targetLanguage})</span>
      <span class="panel-title" style="font-size: 0.85rem; color: rgba(255,255,255,0.6);">
        {translations.length} translations
      </span>
    </div>
    <div class="panel-content">
      {#each translations as item, index}
        <div class="transcription-item">
          <div class="transcription-text">{item.translation.translated_text}</div>
          <div class="transcription-meta">
            Score: {item.translation.score.toFixed(2)} |
            {item.translation.source_lang} → {item.translation.target_lang}
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

  .transcription-text {
    font-size: 0.95rem;
    line-height: 1.4;
    margin-bottom: 0.25rem;
  }

  .transcription-meta {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.5);
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
</style>
