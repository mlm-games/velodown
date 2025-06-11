<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  interface Download {
    id: string;
    url: string;
    status: 'Queued' | 'Downloading' | 'Paused' | 'Completed' | 'Error';
    progress: number;
    file_name: string;
    save_path: string;
  }

  let downloads: Download[] = [];
  let unlistenProgress: Function;
  let unlistenTaskUpdated: Function;

  onMount(async () => {
    try {
      downloads = await invoke<Download[]>('get_all_downloads');
    } catch (e) {
      console.error("Failed to get initial downloads:", e);
    }

    unlistenTaskUpdated = await listen('task_updated', (event: any) => {
      const updatedTask: Download = event.payload;
      const index = downloads.findIndex(d => d.id === updatedTask.id);
      if (index !== -1) {
        // If the task exists, update it
        downloads[index] = updatedTask;
      } else {
        // If it's a new task, add it to the top of the list
        downloads = [updatedTask, ...downloads];
      }
    });

    unlistenProgress = await listen('download_progress', (event: any) => {
      const { id, progress } = event.payload;
      const index = downloads.findIndex(d => d.id === id);
      if (index !== -1) {
        downloads[index].progress = progress;
        // This is a Svelte trick to force the UI to update the array
        downloads = [...downloads];
      }
    });
  });

  onDestroy(() => {
    // Always clean up listeners
    if (unlistenProgress) unlistenProgress();
    if (unlistenTaskUpdated) unlistenTaskUpdated();
  });
</script>

<section>
  <h2>Active Downloads</h2>
  {#if downloads.length === 0}
    <p>The queue is empty. Add a new download to get started.</p>
  {:else}
    <div class="list">
      {#each downloads as download (download.id)}
        <div class="download-item" class:completed={download.status === 'Completed'}>
          <span class="filename">{download.file_name}</span>
          <span class="status">{download.status}</span>
          <div class="progress-bar">
            <div class="progress" style="width: {download.progress}%"></div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  /* (Keep your previous styles, but add this one for completed items) */
  .filename {
    font-weight: bold;
    word-break: break-all;
    display: block;
    margin-bottom: 0.5rem;
  }
  .download-item.completed {
    opacity: 0.7;
    border-left: 4px solid #4CAF50;
  }
    /* Rest of the styles from previous step */
  .list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .download-item {
    background-color: #333;
    padding: 1rem;
    border-radius: 5px;
    border-left: 4px solid #555;
    transition: all 0.2s ease-in-out;
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
