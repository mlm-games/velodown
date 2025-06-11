<!-- src/lib/Settings.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  
  interface AppSettings {
    download_folder: string;
    max_concurrent_downloads: number;
    max_connections_per_download: number;
    auto_start: boolean;
    show_notifications: boolean;
    min_split_size: number;
  }
  
  let settings: AppSettings = {
    download_folder: '',
    max_concurrent_downloads: 4,
    max_connections_per_download: 8,
    auto_start: true,
    show_notifications: true,
    min_split_size: 10485760, // 10MB
  };
  
  let message = '';
  let messageType: 'success' | 'error' = 'success';

  onMount(async () => {
    try {
      settings = await invoke<AppSettings>('get_settings');
    } catch (e) {
      message = 'Could not load settings.';
      messageType = 'error';
    }
  });

  async function chooseFolder() {
    try {
      const folder = await invoke<string>('choose_download_folder');
      settings.download_folder = folder;
    } catch (error) {
      console.error('Failed to choose folder:', error);
    }
  }

  async function saveSettings() {
    try {
      await invoke('update_settings', { settings });
      message = 'Settings saved successfully!';
      messageType = 'success';
    } catch (e) {
      message = `Error saving settings: ${e}`;
      messageType = 'error';
    }
  }
</script>

<section>
  <h2>Settings</h2>
  <form on:submit|preventDefault={saveSettings}>
    <div class="form-group">
      <label for="folder">Default Download Folder</label>
      <div class="folder-selector">
        <input 
          id="folder" 
          type="text" 
          bind:value={settings.download_folder} 
          readonly
          placeholder="Select download folder..."
        />
        <button type="button" on:click={chooseFolder} class="browse-btn">
          Browse...
        </button>
      </div>
    </div>
    
    <div class="form-group">
      <label for="concurrent">Max Concurrent Downloads</label>
      <input 
        id="concurrent" 
        type="number" 
        bind:value={settings.max_concurrent_downloads} 
        min="1" 
        max="10"
      />
      <small>Number of downloads that can run simultaneously (1-10)</small>
    </div>
    
    <div class="form-group">
      <label for="connections">Max Connections Per Download</label>
      <input 
        id="connections" 
        type="number" 
        bind:value={settings.max_connections_per_download} 
        min="1" 
        max="16"
      />
      <small>Number of parallel connections for each download (1-16)</small>
    </div>
    
    <div class="form-group checkbox-group">
      <label>
        <input 
          type="checkbox" 
          bind:checked={settings.auto_start}
        />
        Auto-start downloads
      </label>
      <small>Automatically start downloads when added</small>
    </div>
    
    <div class="form-group checkbox-group">
      <label>
        <input 
          type="checkbox" 
          bind:checked={settings.show_notifications}
        />
        Show notifications
      </label>
      <small>Display notifications when downloads complete</small>
    </div>
    
    <button type="submit" class="save-btn">Save Settings</button>
  </form>
  
  {#if message}
    <div class="message {messageType}">
      {message}
    </div>
  {/if}
</section>

<style>
  section {
    max-width: 600px;
    margin: 0 auto;
  }

  .form-group {
    margin-bottom: 1.5rem;
  }

  label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
  }

  input[type="text"],
  input[type="number"] {
    width: 100%;
    padding: 10px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 4px;
    color: #fff;
    font-size: 14px;
  }

  input[type="number"] {
    max-width: 120px;
  }

  .folder-selector {
    display: flex;
    gap: 8px;
  }

  .folder-selector input {
    flex: 1;
  }

  .browse-btn {
    padding: 10px 20px;
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

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: normal;
  }

  input[type="checkbox"] {
    width: auto;
    cursor: pointer;
  }

  small {
    display: block;
    margin-top: 0.25rem;
    color: #888;
    font-size: 0.875rem;
  }

  .save-btn {
    width: 100%;
    padding: 12px;
    background: #4CAF50;
    border: none;
    border-radius: 4px;
    color: white;
    font-size: 16px;
    cursor: pointer;
    transition: background 0.3s;
  }

  .save-btn:hover {
    background: #45a049;
  }

  .message {
    margin-top: 1rem;
    padding: 12px;
    border-radius: 4px;
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
</style>
