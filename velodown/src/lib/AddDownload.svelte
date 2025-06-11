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
  </form>

  {#if message}
    <div class="message {messageType}">
      {message}
    </div>
  {/if}

  <div class="tips">
    <h3>Tips:</h3>
    <ul>
      <li>Paste URLs directly from your clipboard</li>
      <li>Supports HTTP and HTTPS downloads</li>
      <li>Downloads will be saved to your default folder unless you specify otherwise</li>
      <li>You can also drag and drop links from your browser</li>
    </ul>
  </div>
</section>

<style>
  section {
    max-width: 600px;
    margin: 0 auto;
  }

  .url-input-group {
    margin-bottom: 1rem;
  }

  .url-input {
    width: 100%;
    padding: 12px;
    font-size: 16px;
    border: 2px solid #444;
    border-radius: 8px;
    background: #2a2a2a;
    color: #fff;
    transition: border-color 0.3s;
  }

  .url-input:focus {
    outline: none;
    border-color: #4CAF50;
  }

  .options {
    margin-bottom: 1rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 0.5rem;
    cursor: pointer;
  }

  .path-selector {
    display: flex;
    gap: 8px;
    margin-top: 0.5rem;
  }

  .path-input {
    flex: 1;
    padding: 8px;
    border: 1px solid #444;
    border-radius: 4px;
    background: #2a2a2a;
    color: #fff;
  }

  .browse-btn {
    padding: 8px 16px;
    background: #555;
    border: none;
    border-radius: 4px;
    color: #fff;
    cursor: pointer;
    transition: background 0.3s;
  }

  .browse-btn:hover {
    background: #666;
  }

  .download-btn {
    width: 100%;
    padding: 12px;
    font-size: 16px;
    background: #4CAF50;
    border: none;
    border-radius: 8px;
    color: white;
    cursor: pointer;
    transition: background 0.3s;
  }

  .download-btn:hover:not(:disabled) {
    background: #45a049;
  }

  .download-btn:disabled {
    background: #666;
    cursor: not-allowed;
  }

  .message {
    margin-top: 1rem;
    padding: 12px;
    border-radius: 8px;
    text-align: center;
  }

  .message.success {
    background: #4CAF50;
    color: white;
  }

  .message.error {
    background: #f44336;
    color: white;
  }

  .message.info {
    background: #2196F3;
    color: white;
  }

  .tips {
    margin-top: 2rem;
    padding: 1rem;
    background: #2a2a2a;
    border-radius: 8px;
  }

  .tips h3 {
    margin-bottom: 0.5rem;
    color: #4CAF50;
  }

  .tips ul {
    list-style: none;
    padding: 0;
  }

  .tips li {
    padding: 0.25rem 0;
    padding-left: 1rem;
    position: relative;
  }

  .tips li::before {
    content: "•";
    position: absolute;
    left: 0;
    color: #4CAF50;
  }
</style>
