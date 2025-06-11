<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  interface Download {
    id: string;
    url: string;
    status: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed' | 'verifying';
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

  let downloads: Download[] = [];
  let filter: 'all' | 'active' | 'completed' = 'all';
  let searchQuery = '';
  let unlistenTaskUpdated: Function;
  let unlistenDownloadRemoved: Function;

  $: filteredDownloads = downloads.filter(d => {
    const matchesFilter = 
      filter === 'all' ||
      (filter === 'active' && ['queued', 'downloading', 'paused'].includes(d.status)) ||
      (filter === 'completed' && d.status === 'completed');
    
    const matchesSearch = 
      searchQuery === '' ||
      d.file_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
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
      downloads = await invoke<Download[]>
