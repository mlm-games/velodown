<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  
  let downloadFolder = '';
  let maxConcurrent = 1;
  let message = '';

  onMount(async () => {
    try {
      const settings = await invoke('get_settings');
      downloadFolder = settings.download_folder;
      maxConcurrent = settings.max_concurrent_downloads;
    } catch (e) {
      message = 'Could not load settings.';
    }
  });

  async function saveSettings() {
     try {
      await invoke('update_settings', { settings: { download_folder: downloadFolder, max_concurrent_downloads: maxConcurrent } });
      message = 'Settings saved successfully!';
    } catch (e) {
      message = `Error saving settings: ${e}`;
    }
  }
</script>

<section>
  <h2>Settings</h2>
  <form on:submit|preventDefault={saveSettings}>
    <div class="form-group">
      <label for="folder">Default Download Folder</label>
      <input id="folder" type="text" bind:value={downloadFolder} />
    </div>
    <div class="form-group">
      <label for="concurrent">Max Concurrent Downloads</label>
      <input id="concurrent" type="number" bind:value={maxConcurrent} />
    </div>
    <button type="submit">Save Settings</button>
  </form>
  {#if message}
    <p class="message">{message}</p>
  {/if}
</section>

<style>
  .form-group {
    margin-bottom: 1rem;
  }
  label {
    display: block;
    margin-bottom: 0.25rem;
  }
  input {
    width: 100%;
    padding: 8px;
  }
  .message {
    margin-top: 1rem;
    color: #aaa;
  }
</style>
