<script lang="ts">
  import Button from './Button.svelte';

  let {
    show = false,
    onFinish,
  }: {
    show: boolean;
    onFinish: () => void;
  } = $props();

  let currentStep = $state(0);

  const steps = [
    {
      icon: '🎤',
      title: 'Real-time Translation',
      description: 'Speak into your microphone and see instant translations appear in real-time.',
      hint: 'Click the microphone button in the center to start'
    },
    {
      icon: '⚙️',
      title: 'Configure Settings',
      description: 'Choose your languages, translation mode, and audio source in the settings panel.',
      hint: 'Click the gear icon to access settings'
    },
    {
      icon: '💾',
      title: 'Save Transcripts',
      description: 'Your translation history is automatically saved. Access, search, and export anytime.',
      hint: 'Click the clock icon to view saved transcripts'
    },
    {
      icon: '✨',
      title: 'AI Summaries',
      description: 'Generate intelligent summaries of your translations with key points and action items.',
      hint: 'Click Generate Summary on any saved transcript'
    },
    {
      icon: '🔊',
      title: 'Text-to-Speech',
      description: 'Enable TTS to hear translations spoken aloud with customizable voices.',
      hint: 'Enable TTS from the settings panel'
    }
  ];

  function nextStep() {
    if (currentStep < steps.length - 1) {
      currentStep++;
    } else {
      finish();
    }
  }

  function prevStep() {
    if (currentStep > 0) {
      currentStep--;
    }
  }

  function finish() {
    // Save that user has completed onboarding
    localStorage.setItem('auralis-onboarding-completed', 'true');
    onFinish();
  }

  function skip() {
    finish();
  }

  // Don't show if already completed
  $effect(() => {
    const completed = localStorage.getItem('auralis-onboarding-completed');
    if (completed && show) {
      onFinish();
    }
  });
</script>

{#if show}
  <div class="onboarding-backdrop" onclick={skip}>
    <div class="onboarding-card" onclick={(e) => e.stopPropagation()}>
      <div class="onboarding-header">
        <div class="onboarding-progress">
          {#each steps as _, i}
            <div class="progress-dot" class:active={i === currentStep} class:completed={i < currentStep}></div>
          {/each}
        </div>
        <Button variant="icon" size="sm" onclick={skip} title="Close">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </Button>
      </div>

      <div class="onboarding-content">
        <div class="step-icon">{steps[currentStep].icon}</div>
        <h2 class="step-title">{steps[currentStep].title}</h2>
        <p class="step-description">{steps[currentStep].description}</p>
        <div class="step-hint">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
          <span>{steps[currentStep].hint}</span>
        </div>
      </div>

      <div class="onboarding-footer">
        <Button variant="ghost" size="sm" onclick={skip}>Skip Tour</Button>
        <div class="footer-actions">
          {#if currentStep > 0}
            <Button variant="secondary" size="sm" onclick={prevStep}>Back</Button>
          {/if}
          <Button variant="primary" size="sm" onclick={nextStep}>
            {currentStep === steps.length - 1 ? 'Get Started' : 'Next'}
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .onboarding-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-overlay);
    animation: fadeIn 0.2s ease;
  }

  .onboarding-card {
    width: 90%;
    max-width: 480px;
    background: var(--bg-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    padding: var(--space-xl);
    animation: slideUp 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .onboarding-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-lg);
  }

  .onboarding-progress {
    display: flex;
    gap: 8px;
  }

  .progress-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
    transition: all 0.3s ease;
  }

  .progress-dot.active {
    background: var(--accent);
    transform: scale(1.3);
  }

  .progress-dot.completed {
    background: var(--accent);
  }

  .onboarding-content {
    text-align: center;
    margin-bottom: var(--space-xl);
  }

  .step-icon {
    font-size: 48px;
    margin-bottom: var(--space-md);
  }

  .step-title {
    font-size: var(--font-size-xl);
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 var(--space-sm) 0;
  }

  .step-description {
    font-size: var(--font-size-md);
    color: var(--text-secondary);
    line-height: 1.6;
    margin: 0 0 var(--space-md) 0;
  }

  .step-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    background: rgba(99, 140, 255, 0.1);
    border: 1px solid rgba(99, 140, 255, 0.2);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    color: var(--accent);
  }

  .onboarding-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .footer-actions {
    display: flex;
    gap: var(--space-sm);
  }
</style>
