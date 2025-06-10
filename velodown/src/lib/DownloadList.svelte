<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  interface Download {
    id: string;
    url: string;
    status: 'Downloading' | 'Completed' | 'Error';
    progress: number;
  }

  let downloads: Download[] = [];
  let unlistenProgress: Function;
  let unlistenStatus: Function;

  // onMount runs when the component is first created.
  // It's the perfect place to start listening for backend events.
  onMount(async () => {
    // Listen for progress updates from Rust
    unlistenProgress = await listen('download_progress', (event: any) => {
      const { id, progress } = event.payload;
      const index = downloads.findIndex(d => d.id === id);
      if (index !== -1) {
        downloads[index].progress = progress;
        // This is a Svelte trick to force the UI to update
        downloads = [...downloads];
      }
    });

    // Listen for status changes (like when a new download is added)
    unlistenStatus = await listen('download_status_changed', (event: any) => {
      const newDownload: Download = event.payload;
      const index = downloads.findIndex(d => d.id === newDownload.id);
      if (index !== -1) {
        downloads[index] = newDownload;
      } else {
        downloads.push(newDownload);
      }
       downloads = [...downloads];
    });
  });

  // onDestroy runs when the component is removed.
  // It's crucial to clean up our listeners to prevent memory leaks.
  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenStatus) unlistenStatus();
  });

</script>

<section>
  <h2>Active Downloads</h2>
  {#if downloads.length === 0}
    <p>No active downloads.</p>
  {:else}
    <div class="list">
      {#each downloads as download (download.id)}
        <div class="download-item">
          <span class="url">{download.url}</span>
          <span class="status">{download.status}</span>
          <div class="progress-bar">
            <div class="progress" style="width: {download.progress}%" />
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .download-item {
    background-color: #333;
    padding: 1rem;
    border-radius: 5px;
  }
  .url {
    word-break: break-all;
    display: block;
    margin-bottom: 0.5rem;
  }
  .status {
    font-style: italic;
    color: #ccc;
    font-size: 0.9em;
  }
  .progress-bar {
    width: 100%;
    background-color: #555;
    border-radius: 5px;
    height: 10px;
    margin-top: 0.5rem;
    overflow: hidden;
  }
  .progress {
    height: 100%;
    background-color: #4CAF50;
    border-radius: 5px;
  }
</style>
