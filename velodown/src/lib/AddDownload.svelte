<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let downloadUrl = '';
  let message = '';

  async function addDownload() {
    if (!downloadUrl) {
      message = 'Please enter a URL.';
      return;
    }
    try {
      // "invoke" is how you call a Rust function from your Svelte code.
      await invoke('add_download', { url: downloadUrl });
      message = `Download started for: ${downloadUrl}`;
      downloadUrl = ''; // Clear the input
    } catch (error) {
      message = `Error: ${error}`;
    }
  }
</script>

<section>
  <h2>Add New Download</h2>
  <form on:submit|preventDefault={addDownload}>
    <input
      type="text"
      bind:value={downloadUrl}
      placeholder="Enter file URL..."
    />
    <button type="submit">Download</button>
  </form>
  {#if message}
    <p class="message">{message}</p>
  {/if}
</section>

<style>
  input {
    width: 70%;
    padding: 8px;
    margin-right: 10px;
  }
  .message {
    margin-top: 1rem;
    color: #aaa;
  }
</style>
