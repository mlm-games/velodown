<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  
  interface AppSettings {
    downloadFolder: string;
    maxConcurrentDownloads: number;
    maxConnectionsPerDownload: number;
    autoStart: boolean;
    showNotifications: boolean;
    autoResumeDownloads: boolean;
    maxResumeAttempts: number;
    resumeDelaySeconds: number;
    minFailDurationSeconds: number;
  }
  
  let settings: AppSettings = {
    downloadFolder: '',
    maxConcurrentDownloads: 4,
    maxConnectionsPerDownload: 8,
    autoStart: true,
    showNotifications: true,
    autoResumeDownloads: true,
    maxResumeAttempts: 10,
    resumeDelaySeconds: 0.25,
    minFailDurationSeconds: 2,
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
    if (!settings) return;
    try {
      const folder = await invoke<string>('choose_download_folder');
      settings.downloadFolder = folder;
    } catch (error) {
      console.error('Failed to choose folder:', error);
    }
  }

  async function saveSettings() {
    if (!settings) return;
    try {
      await invoke('update_settings', { settings });
      message = 'Settings saved successfully!';
      messageType = 'success';
      setTimeout(() => message = '', 3000);
    } catch (e) {
      message = `Error saving settings: ${e}`;
      messageType = 'error';
    }
  }
</script>

<section>
  <h2>Settings</h2>
  
  {#if !settings}
    <p>Loading settings...</p>
  {:else}
    <form on:submit|preventDefault={saveSettings}>
      <!-- Download Location -->
      <div class="form-group">
        <label for="folder">Default Download Folder</label>
        <div class="folder-selector">
          <input id="folder" type="text" bind:value={settings.downloadFolder} readonly />
          <button type="button" on:click={chooseFolder} class="browse-btn">Browse...</button>
        </div>
      </div>
      
      <!-- General Settings -->
      <div class="grid-2">
        <div class="form-group">
          <label for="concurrent">Max Concurrent Downloads</label>
          <input id="concurrent" type="number" bind:value={settings.maxConcurrentDownloads} min="1" max="10" />
        </div>
        <div class="form-group">
          <label for="connections">Max Connections Per Download</label>
          <input id="connections" type="number" bind:value={settings.maxConnectionsPerDownload} min="1" max="16" />
        </div>
      </div>
      
      <!-- Checkbox Options -->
      <div class="grid-2">
        <div class="form-group checkbox-group">
          <label><input type="checkbox" bind:checked={settings.autoStart}/> Auto-start downloads</label>
        </div>
        <div class="form-group checkbox-group">
          <label><input type="checkbox" bind:checked={settings.showNotifications}/> Show notifications</label>
        </div>
      </div>

      <hr />
      
      <h3 class="section-title">Auto-Resume Failed Downloads</h3>
      
      <div class="form-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.autoResumeDownloads} />
          Automatically try to resume broken downloads
        </label>
      </div>

      {#if settings.autoResumeDownloads}
        <div class="grid-3">
          <div class="form-group">
            <label for="resume-attempts">Max Resume Attempts</label>
            <input id="resume-attempts" type="number" bind:value={settings.maxResumeAttempts} min="1" max="20" />
          </div>
          <div class="form-group">
            <label for="resume-delay">Delay (seconds)</label>
            <input id="resume-delay" type="number" bind:value={settings.resumeDelaySeconds} min="5" max="300" />
          </div>
          <div class="form-group">
            <label for="quick-fail">Quick Fail (seconds)</label>
            <input id="quick-fail" type="number" bind:value={settings.minFailDurationSeconds} min="5" max="60" />
            <small>Don't retry if a download fails faster than this.</small>
          </div>
        </div>
      {/if}
      
      <button type="submit" class="save-btn">Save Settings</button>
    </form>
  {/if}
  
  {#if message}
    <div class="message {messageType}">{message}</div>
  {/if}
</section>

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

  hr { border: none; border-top: 1px solid #444; margin: 2rem 0; }
  .section-title { margin-bottom: 1.5rem; font-size: 1.25rem; }
  .grid-2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 1.5rem; }
  .grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.5rem; }
  small { font-size: 0.8rem; color: #888; margin-top: 0.25rem; display: block; }

</style>
