<!-- src/lib/DownloadList.svelte -->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  // --- FIX: Use camelCase to match the Rust struct ---
  interface Download {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed' | 'verifying';
    progress: number;
    fileName: string;       // Corrected
    savePath: string;       // Corrected
    totalSize: number;      // Corrected
    downloadedSize: number; // Corrected
    speed: number;
    timeRemaining: number | null; // Corrected
    resumeCapability: boolean;  // Corrected
    errorMessage: string | null;  // Corrected
    createdAt: string;          // Corrected
    completedAt: string | null; // Corrected
    fileType: string;           // Corrected
  }

  let downloads: Download[] = [];
  let filter: 'all' | 'active' | 'completed' = 'all';
  let searchQuery = '';
  let unlistenTaskUpdated: Function;
  let unlistenDownloadRemoved: Function;

  $: filteredDownloads = downloads.filter(d => {
    const matchesFilter = 
      filter === 'all' ||
      (filter === 'active' && ['queued', 'downloading', 'paused', 'verifying'].includes(d.status)) ||
      (filter === 'completed' && d.status === 'completed');
    
    // FIX: Search camelCase properties
    const matchesSearch = 
      searchQuery === '' ||
      d.fileName.toLowerCase().includes(searchQuery.toLowerCase()) ||
      d.url.toLowerCase().includes(searchQuery.toLowerCase());
    
    return matchesFilter && matchesSearch;
  });

  onMount(async () => {
    await loadDownloads();

    unlistenTaskUpdated = await listen('task_updated', (event: any) => {
      const updatedTask: Download = event.payload;
      const index = downloads.findIndex(d => d.id === updatedTask.id);
      if (index !== -1) {
        downloads[index] = updatedTask;
      } else {
        downloads = [updatedTask, ...downloads];
      }
      downloads = downloads; // Trigger Svelte reactivity
    });

    unlistenDownloadRemoved = await listen('download_removed', (event: any) => {
      const id = event.payload;
      downloads = downloads.filter(d => d.id !== id);
    });
  });

  onDestroy(() => {
    if (unlistenTaskUpdated) unlistenTaskUpdated();
    if (unlistenDownloadRemoved) unlistenDownloadRemoved();
  });

  async function loadDownloads() {
    try {
      downloads = await invoke<Download[]>('get_all_downloads');
    } catch (error) {
      console.error('Failed to load downloads:', error);
    }
  }

  async function pauseDownload(id: string) {
    try {
      await invoke('pause_download', { id });
    } catch (error) {
      console.error('Failed to pause download:', error);
    }
  }

  async function resumeDownload(id: string) {
    try {
      await invoke('resume_download', { id });
    } catch (error) {
      console.error('Failed to resume download:', error);
    }
  }

  async function cancelDownload(id: string) {
    if (confirm('Are you sure you want to cancel this download?')) {
      try {
        await invoke('cancel_download', { id });
      } catch (error) {
        console.error('Failed to cancel download:', error);
      }
    }
  }

  async function openFile(savePath: string, fileName: string) {
    try {
      // Use the corrected command that takes separate arguments
      await invoke('open_file', { savePath, fileName });
    } catch (error) {
      console.error('Failed to open file:', error);
    }
  }

  async function openFolder(path: string) {
    try {
      await invoke('open_folder', { path });
    } catch (error) {
      console.error('Failed to open folder:', error);
    }
  }

  // --- FIX: Robust formatting functions ---
  function formatBytes(bytes: number): string {
    if (!bytes || bytes <= 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatSpeed(bytesPerSecond: number): string {
    if (!bytesPerSecond || bytesPerSecond <= 0) return '0 B/s';
    return formatBytes(bytesPerSecond) + '/s';
  }

  function formatTime(seconds: number | null): string {
    if (seconds === null || seconds <= 0 || !isFinite(seconds)) return '∞';
    if (seconds > 86400 * 30) return '>30d'; 
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'downloading': return '⬇️';
      case 'paused': return '⏸️';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'queued': return '⏳';
      case 'verifying': return '🔍';
      default: return '❓';
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'downloading': return '#4CAF50';
      case 'paused': return '#FF9800';
      case 'completed': return '#2196F3';
      case 'failed': return '#f44336';
      case 'queued': return '#9E9E9E';
      case 'verifying': return '#9C27B0';
      default: return '#757575';
    }
  }
</script>

<section>
  <div class="controls">
    <div class="filters">
      <button class:active={filter === 'all'} on:click={() => filter = 'all'}>All</button>
      <button class:active={filter === 'active'} on:click={() => filter = 'active'}>Active</button>
      <button class:active={filter === 'completed'} on:click={() => filter = 'completed'}>Completed</button>
    </div>
    <input type="search" bind:value={searchQuery} placeholder="Search downloads..." class="search-input" />
  </div>

  <div class="downloads-list">
    {#if filteredDownloads.length === 0}
      <div class="empty-state">
        <p>No downloads found</p>
        <a href="/add" class="add-link">Add a new download</a>
      </div>
    {:else}
      {#each filteredDownloads as download (download.id)}
        <div class="download-item" style="--status-color: {getStatusColor(download.status)}">
          <div class="download-header">
            <div class="file-info">
              <span class="status-icon">{getStatusIcon(download.status)}</span>
              <div>
                <h3 class="file-name">{download.fileName}</h3>
                <p class="file-details">
                  {download.fileType} • {formatBytes(download.totalSize)}
                  {#if download.status === 'downloading'}
                    • {formatSpeed(download.speed)} • {formatTime(download.timeRemaining)}
                  {/if}
                </p>
              </div>
            </div>
            
            <div class="actions">
              {#if download.status === 'downloading'}
                <button on:click={() => pauseDownload(download.id)} title="Pause">⏸️</button>
              {:else if download.status === 'paused' || download.status === 'failed'}
                <button on:click={() => resumeDownload(download.id)} title="Resume">▶️</button>
              {/if}
              
              {#if download.status === 'completed'}
                <!-- FIX: Access camelCase properties -->
                <button on:click={() => openFile(download.savePath, download.fileName)} title="Open File">📄</button>
              {/if}
              
              <!-- FIX: Access camelCase property -->
              <button on:click={() => openFolder(download.savePath)} title="Open Folder">📁</button>
              
              {#if download.status !== 'completed'}
                <button on:click={() => cancelDownload(download.id)} title="Cancel" class="cancel-btn">❌</button>
              {/if}
            </div>
          </div>
          
          {#if download.status === 'downloading' || download.status === 'paused'}
            <div class="progress-container">
              <div class="progress-bar">
                <div class="progress-fill" style="width: {download.progress}%"></div>
              </div>
              <span class="progress-text">{download.progress.toFixed(1)}%</span>
            </div>
          {/if}
          
          <!-- FIX: Access camelCase property -->
          {#if download.errorMessage}
            <p class="error-message">{download.errorMessage}</p>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  section {
    max-width: 1000px;
    margin: 0 auto;
  }

  .controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    gap: 1rem;
  }

  .filters {
    display: flex;
    gap: 0.5rem;
  }

  .filters button {
    padding: 8px 16px;
    background: #333;
    border: 1px solid #555;
    border-radius: 4px;
    color: #fff;
    cursor: pointer;
    transition: all 0.3s;
  }

  .filters button:hover {
    background: #444;
  }

  .filters button.active {
    background: #4CAF50;
    border-color: #4CAF50;
  }

  .search-input {
    flex: 1;
    max-width: 300px;
    padding: 8px 12px;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 4px;
    color: #fff;
  }

  .downloads-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .empty-state {
    text-align: center;
    padding: 3rem;
    color: #888;
  }

  .add-link {
    color: #4CAF50;
    text-decoration: none;
    margin-top: 1rem;
    display: inline-block;
  }

  .add-link:hover {
    text-decoration: underline;
  }

  .download-item {
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 8px;
    padding: 1rem;
    border-left: 4px solid var(--status-color);
  }

  .download-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
  }

  .status-icon {
    font-size: 1.5rem;
  }

  .file-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 500;
    word-break: break-word;
  }

  .file-details {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: #888;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .actions button {
    padding: 6px 10px;
    background: #333;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.3s;
  }

  .actions button:hover {
    background: #444;
    transform: scale(1.1);
  }

  .cancel-btn:hover {
    background: #f44336 !important;
  }

  .progress-container {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-top: 0.75rem;
  }

  .progress-bar {
    flex: 1;
    height: 8px;
    background: #444;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #4CAF50;
    transition: width 0.3s ease;
  }

  .progress-text {
    font-size: 0.875rem;
    color: #888;
    min-width: 50px;
    text-align: right;
  }

  .error-message {
    margin: 0.5rem 0 0 0;
    color: #f44336;
    font-size: 0.875rem;
  }
</style>
