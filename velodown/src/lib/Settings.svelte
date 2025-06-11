<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  
  interface AppSettings {
    downloadFolder: string;
    maxConcurrentDownloads: number;
    maxConnectionsPerDownload: number;
    autoStart: boolean;
    showNotifications: boolean;
    minSplitSize: number;
  }
  
  let settings: AppSettings = {
    downloadFolder: '',
    maxConcurrentDownloads: 4,
    maxConnectionsPerDownload: 8,
    autoStart: true,
    showNotifications: true,
    minSplitSize: 10485760,
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
      settings.downloadFolder = folder;
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
          bind:value={settings.downloadFolder} 
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
        bind:value={settings.maxConcurrentDownloads} 
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
        bind:value={settings.maxConnectionsPerDownload} 
        min="1" 
        max="16"
      />
      <small>Number of parallel connections for each download (1-16)</small>
    </div>
    
    <div class="form-group checkbox-group">
      <label>
        <input 
          type="checkbox" 
          bind:checked={settings.autoStart}
        />
        Auto-start downloads
      </label>
      <small>Automatically start downloads when added</small>
    </div>
    
    <div class="form-group checkbox-group">
      <label>
        <input 
          type="checkbox" 
          bind:checked={settings.showNotifications}
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

<!-- STYLES -->
<style>
  section { max-width: 600px; margin: 0 auto; }
  .form-group { margin-bottom: 1.5rem; }
  label { display: block; margin-bottom: 0.5rem; font-weight: 500; }
  input[type="text"], input[type="number"] {
    width: 100%; padding: 10px; background: #2a2a2a;
    border: 1px solid #444; border-radius: 4px;
    color: #fff; font-size: 14px;
  }
  input[type="number"] { max-width: 120px; }
  .folder-selector { display: flex; gap: 8px; }
  .folder-selector input { flex: 1; }
  .browse-btn {
    padding: 10px 20px; background: #555; border: none;
    border-radius: 4px; color: #fff; cursor: pointer;
    transition: background 0.3s;
  }
  .browse-btn:hover { background: #666; }
  .checkbox-group label {
    display: flex; align-items: center; gap: 8px;
    cursor: pointer; font-weight: normal;
  }
  input[type="checkbox"] { width: auto; cursor: pointer; }
  small {
    display: block; margin-top: 0.25rem; color: #888;
    font-size: 0.875rem;
  }
  .save-btn {
    width: 100%; padding: 12px; background: #4CAF50;
    border: none; border-radius: 4px; color: white;
    font-size: 16px; cursor: pointer; transition: background 0.3s;
  }
  .save-btn:hover { background: #45a049; }
  .message {
    margin-top: 1rem; padding: 12px; border-radius: 4px;
    text-align: center;
  }
  .message.success { background: #4CAF50; color: white; }
  .message.error { background: #f44336; color: white; }
</style>
