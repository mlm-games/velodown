<!-- src/lib/AddDownload.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';

  interface DownloadTask {
    id: string;
    url: string;
    status: string;
    progress: number;
    file_name: string;
    save_path: string;
    total_size: number;
    downloaded_size: number;
    speed: number;
    time_remaining: number | null;
    resume_capability: boolean;
    error_message: string | null;
    created_at: string;
    completed_at: string | null;
    file_type: string;
  }

  let downloadUrl = '';
  let customPath = '';
  let useCustomPath = false;
  let message = '';
  let messageType: 'success' | 'error' | 'info' = 'info';
  let isValidating = false;
  let unlisten: (() => void) | null = null;

  onMount(() => {
    // Listen for URLs from command line
    listen('cli-url', (event: any) => {
      downloadUrl = event.payload;
      validateAndAdd();
    }).then(unlistenFn => {
      unlisten = unlistenFn;
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function chooseFolder() {
    try {
      const folder = await invoke<string>('choose_download_folder');
      customPath = folder;
      useCustomPath = true;
    } catch (error) {
      console.error('Failed to choose folder:', error);
    }
  }

  async function validateAndAdd() {
    if (!downloadUrl) {
      message = 'Please enter a URL.';
      messageType = 'error';
      return;
    }

    isValidating = true;
    message = 'Validating URL...';
    messageType = 'info';

    try {
      const task = await invoke<DownloadTask>('add_download', { 
        url: downloadUrl,
        customPath: useCustomPath ? customPath : null
      });
      
      message = `Download started: ${task.file_name}`;
      messageType = 'success';
      downloadUrl = '';
      customPath = '';
      useCustomPath = false;
      
      // Redirect to downloads page after 1 second
      setTimeout(() => {
        goto('/');
      }, 1000);
    } catch (error) {
      message = `Error: ${error}`;
      messageType = 'error';
    } finally {
      isValidating = false;
    }
  }

  function handlePaste(event: ClipboardEvent) {
    const text = event.clipboardData?.getData('text');
    if (text && (text.startsWith('http://') || text.startsWith('https://'))) {
      downloadUrl = text;
    }
  }
</script>

<section>
  <h2>Add New Download</h2>
  
  <form on:submit|preventDefault={validateAndAdd}>
    <div class="url-input-group">
      <input
        type="text"
        bind:value={downloadUrl}
        placeholder="Enter or paste download URL..."
        on:paste={handlePaste}
        disabled={isValidating}
        class="url-input"
      />
    </div>

    <div class="options">
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={useCustomPath} />
        Use custom download location
      </label>
      
      {#if useCustomPath}
        <div class="path-selector">
          <input
            type="text"
            bind:value={customPath}
            placeholder="Download location..."
            readonly
            class="path-input"
          />
          <button type="button" on:click={chooseFolder} class="browse-btn">
            Browse...
          </button>
        </div>
      {/if}
    </div>

    <div class="actions">
      <button type="submit" disabled={isValidating} class="download-btn">
        {isValidating ? 'Processing...' : 'Start Download'}
      </button>
    </div>
  
