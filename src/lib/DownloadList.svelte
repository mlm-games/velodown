<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
    import { error } from '@sveltejs/kit';

  interface Download {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed' | 'verifying' | 'retrying';
    progress: number;
    fileName: string;      
    savePath: string;      
    totalSize: number;     
    downloadedSize: number;
    speed: number;
    timeRemaining: number | null;
    resumeCapability: boolean; 
    errorMessage: string | null; 
    createdAt: string;         
    completedAt: string | null;
    fileType: string;          
    resumeAttempts: number; 
  }

  let downloads: Download[] = [];
  let filter: 'all' | 'active' | 'completed' = 'all';
  let searchQuery = '';
  let unlistenTaskUpdated: (() => void) | undefined;
  let unlistenDownloadRemoved: (() => void) | undefined;
  let contextMenu: { x: number; y: number; downloadId: string } | null = null;
  let contextMenuRef: HTMLDivElement;
  let previouslyFocusedElement: HTMLElement | null = null;

  $: filteredDownloads = downloads.filter(d => {
    const matchesFilter = 
      filter === 'all' ||
      (filter === 'active' && ['queued', 'downloading', 'paused', 'verifying', 'retrying'].includes(d.status)) ||
      (filter === 'completed' && d.status === 'completed');
    
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
      downloads = [...downloads];
    });

    unlistenDownloadRemoved = await listen('download_removed', (event: any) => {
      const id = event.payload;
      downloads = downloads.filter(d => d.id !== id);
    });
  
  });

  onDestroy(() => {
    if (unlistenTaskUpdated) unlistenTaskUpdated();
    if (unlistenDownloadRemoved) unlistenDownloadRemoved();
    
  
  
    document.removeEventListener('click', handleGlobalClick, { capture: true });
    document.removeEventListener('contextmenu', handleGlobalRightClick, { capture: true });
    document.removeEventListener('keydown', handleContextMenuKeyDown);
  });


  $: if (contextMenu && contextMenuRef) {
    document.addEventListener('click', handleGlobalClick, { capture: true });
    document.addEventListener('contextmenu', handleGlobalRightClick, { capture: true });
    document.addEventListener('keydown', handleContextMenuKeyDown);
    
  
    const firstButton = contextMenuRef.querySelector('button[role="menuitem"]') as HTMLElement;
    if (firstButton) {
      firstButton.focus();
    }
  } else if (!contextMenu) {
    document.removeEventListener('click', handleGlobalClick, { capture: true });
    document.removeEventListener('contextmenu', handleGlobalRightClick, { capture: true });
    document.removeEventListener('keydown', handleContextMenuKeyDown);
    if (previouslyFocusedElement) {
      previouslyFocusedElement.focus();
      previouslyFocusedElement = null;
    }
  }


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

  async function removeDownloadFromList(id: string) {
    if (confirm('Remove this download from the list?')) {
      try {
        await invoke('remove_download', { id });
      
      } catch (error) {
        console.error('Failed to remove download:', error);
      }
    }
  }

  async function deleteDownloadAndFile(id: string) {
    if (confirm('Delete this download and its file? This cannot be undone.')) {
      try {
        await invoke('delete_download_with_file', { id });
      
      } catch (error) {
        console.error('Failed to delete download:', error);
      }
    }
  }


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

  function getStatusIcon(status: Download['status']): string {
    switch (status) {
      case 'downloading': return '⬇️';
      case 'paused': return '⏸️';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'queued': return '⏳';
      case 'verifying': return '🔍';
      case 'retrying': return '🔄';
      default: return '❓';
    }
  }

  function getStatusColor(status: Download['status']): string {
    switch (status) {
      case 'downloading': return '#4CAF50';
      case 'paused': return '#FF9800';
      case 'completed': return '#2196F3';
      case 'failed': return '#f44336';
      case 'queued': return '#9E9E9E';
      case 'verifying': return '#9C27B0';
      case 'retrying': return '#FFC107';
      default: return '#757575';
    }
  }


  function showContextMenu(event: MouseEvent | KeyboardEvent, downloadId: string) {
    event.preventDefault();
    
    if (event.currentTarget instanceof HTMLElement) {
        previouslyFocusedElement = event.currentTarget;
    }

    let x: number, y: number;
    if (event instanceof KeyboardEvent && event.currentTarget instanceof HTMLElement) {
        const rect = event.currentTarget.getBoundingClientRect();
        x = rect.left;
        y = rect.bottom;
    } else if (event instanceof MouseEvent) {
        x = event.clientX;
        y = event.clientY;
    } else {
      
        x = window.innerWidth / 2;
        y = window.innerHeight / 2;
    }

    contextMenu = { x, y, downloadId };
  }

  function hideContextMenu() {
    contextMenu = null;
  }

  function handleDownloadItemKeyDown(event: KeyboardEvent, downloadId: string) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      showContextMenu(event, downloadId);
    }
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      showContextMenu(event, downloadId);
    }
  }


  function handleGlobalClick(event: MouseEvent) {
    if (contextMenuRef && !contextMenuRef.contains(event.target as Node)) {
      hideContextMenu();
    }
  }
  
  function handleGlobalRightClick(event: MouseEvent) {
  
  
  
    if (contextMenuRef && !contextMenuRef.contains(event.target as Node)) {
      hideContextMenu();
    }
  
  }

  function handleContextMenuKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      hideContextMenu();
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
        <div 
          class="download-item" 
          style="--status-color: {getStatusColor(download.status)}" 
          on:contextmenu={(e) => showContextMenu(e, download.id)}
          on:keydown={(e) => handleDownloadItemKeyDown(e, download.id)}
          role="button"
          tabindex="0"
          aria-haspopup="menu"
          aria-label={`Actions for ${download.fileName}`}
        >
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
                <button on:click|stopPropagation={() => pauseDownload(download.id)} title="Pause">⏸️</button>
              {:else if (download.status === 'paused' || download.status === 'failed') && download.resumeCapability}
                <button on:click|stopPropagation={() => resumeDownload(download.id)} title="Resume">▶️</button>
              {/if}
              
              {#if download.status === 'completed'}
                <button on:click|stopPropagation={() => openFile(download.savePath, download.fileName)} title="Open File">📄</button>
              {/if}
              
              <button on:click|stopPropagation={() => openFolder(download.savePath)} title="Open Folder">📁</button>
              
              {#if download.status !== 'completed'}
                <button on:click|stopPropagation={() => cancelDownload(download.id)} title="Cancel" class="cancel-btn">❌</button>
              {/if}
            </div>
          </div>
          
          {#if ['downloading', 'paused', 'verifying', 'retrying'].includes(download.status)}
            <div class="progress-container">
              <div class="progress-bar">
                <div class="progress-fill" style="width: {download.progress}%"></div>
              </div>
              <span class="progress-text">{download.progress.toFixed(1)}%</span>
            </div>
          {/if}
          
          {#if download.errorMessage}
            <p class="error-message">{download.errorMessage}</p>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Context Menu -->
  {#if contextMenu}
    {@const selectedDownload = downloads.find(d => d.id === contextMenu!.downloadId)}
    {#if selectedDownload}
      <div 
        bind:this={contextMenuRef}
        class="context-menu" 
        style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
        role="menu"
        aria-label={`Actions for ${selectedDownload.fileName}`}
      >
        {#if selectedDownload.status === 'completed'}
          <button role="menuitem" on:click={() => { openFile(selectedDownload.savePath, selectedDownload.fileName); hideContextMenu(); }}>
            📄 Open File
          </button>
        {/if}
        <button role="menuitem" on:click={() => { openFolder(selectedDownload.savePath || ''); hideContextMenu(); }}>
          📁 Open Folder
        </button>
        <hr />
        <button role="menuitem" on:click={() => { removeDownloadFromList(selectedDownload.id); hideContextMenu(); }}>
          🗑️ Remove from List
        </button>
        <button role="menuitem" on:click={() => { deleteDownloadAndFile(selectedDownload.id); hideContextMenu(); }} class="danger">
          ❌ Delete with File
        </button>
      </div>
    {:else}
      <!-- Fallback if download not found, should ideally not happen -->
      <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px; border: 1px solid red; padding: 5px;">
        Error: Download not found.
      </div>
      <script>console.error("Context menu error: Download ID not found"")</script>
      {@debug contextMenu, downloads}
    {/if}
  {/if}
</section>

<style>
  .download-item:focus {
    outline: 2px solid var(--status-color, #4CAF50);
    outline-offset: 2px;
  }

  .context-menu {
    position: fixed;
    background: #3a3a3a;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    padding: 0.5rem 0;
    z-index: 1000;
    min-width: 200px;
    color: #eee;
  }
  .context-menu button[role="menuitem"] {
    display: block;
    width: 100%;
    padding: 0.6rem 1rem;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
    color: #eee;
    font-size: 0.9rem;
  }
  .context-menu button[role="menuitem"]:hover,
  .context-menu button[role="menuitem"]:focus {
    background-color: #4CAF50;
    color: #fff;
    outline: none;
  }
  .context-menu hr {
    border: none;
    border-top: 1px solid #555;
    margin: 0.5rem 0;
  }
  .context-menu button.danger:hover,
  .context-menu button.danger:focus {
    background-color: #f44336;
    color: #fff;
  }
  .context-menu button.danger {
    color: #f44336;
  }

  section {
    max-width: 1000px;
    margin: 0 auto;
    padding: 1rem;
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
    border-left: 5px solid var(--status-color, grey);
    border-radius: 8px;
    padding: 1rem;
    cursor: default;
    transition: box-shadow 0.2s ease-in-out;
  }
  .download-item:hover {
      box-shadow: 0 0 8px rgba(var(--status-color, #757575), 0.5);
  }


  .download-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.5rem;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-grow: 1;
    min-width: 0;
  }

  .status-icon {
    font-size: 1.5rem;
  }

  .file-info > div {
    min-width: 0;
  }

  .file-name {
    margin: 0 0 0.25rem 0;
    font-size: 1.1rem;
    color: #eee;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-details {
    margin: 0;
    font-size: 0.85rem;
    color: #aaa;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .actions button {
    background: #383838;
    border: 1px solid #555;
    color: #ddd;
    cursor: pointer;
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    font-size: 0.9rem;
    transition: background-color 0.2s;
  }

  .actions button:hover {
    background: #4a4a4a;
  }

  .actions button.cancel-btn {
    color: #f44336;
    border-color: #f44336;
  }
  .actions button.cancel-btn:hover {
    background: #f44336;
    color: #fff;
  }

  .progress-container {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.75rem;
  }

  .progress-bar {
    flex-grow: 1;
    height: 8px;
    background-color: #444;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background-color: var(--status-color, #4CAF50);
    transition: width 0.3s ease;
    border-radius: 4px;
  }

  .progress-text {
    font-size: 0.8rem;
    color: #bbb;
  }

  .error-message {
    color: #f44336;
    font-size: 0.85rem;
    margin-top: 0.5rem;
    background-color: rgba(244, 67, 54, 0.1);
    padding: 0.3rem 0.5rem;
    border-radius: 4px;
  }
</style>