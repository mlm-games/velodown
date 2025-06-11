<!-- src/lib/AddDownload.svelte -->
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
    import { onMount } from 'svelte';

	let url = '';
	let customPath = '';

    // NEW: Update the interface to include finalUrl
	let downloadInfo: { finalUrl: string; fileName: string; totalSize: number | null; fileType: string } | null = null;
	let error = '';
	let isLoading = false;
    let defaultDownloadFolder = '...';

    onMount(async () => {
        try {
            // NOTE: Your AppSettings struct uses snake_case, but the #[serde(rename_all = "camelCase")] on it
            // means the frontend receives camelCase keys.
            const settings = await invoke<{ downloadFolder: string }>('get_settings');
            defaultDownloadFolder = settings.downloadFolder;
        } catch (e) {
            console.error("Could not load settings:", e);
            defaultDownloadFolder = "Error loading path";
        }
    });

    function formatBytes(bytes: number | null): string {
        if (bytes === null || bytes === 0) return 'Unknown Size';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

	async function fetchInfo() {
		if (!url) return;
		error = '';
		isLoading = true;
		downloadInfo = null;
		
		try {
			const info = await invoke<typeof downloadInfo>('get_download_info', { url });
			downloadInfo = info;
		} catch (e) {
			error = e as string;
		} finally {
			isLoading = false;
		}
	}

	async function handleAddDownload() {
		if (!downloadInfo) return;
		error = '';
		isLoading = true;

		const payload = {
			url: downloadInfo.finalUrl, 
			fileName: downloadInfo.fileName,
			totalSize: downloadInfo.totalSize,
			customPath: customPath || null,
		};

		try {
			await invoke('add_download', { payload });
			await goto('/');
		} catch (e) {
			error = e as string;
		} finally {
			isLoading = false;
		}
	}

	async function chooseFolder() {
		try {
			const selectedPath = await invoke<string>('choose_download_folder');
			customPath = selectedPath;
		} catch (e) {
			console.error('Folder selection failed:', e);
			if (e !== 'Dialog was cancelled' && e !== 'No folder selected') {
				error = e as string;
			}
		}
	}

    function handlePaste(event: ClipboardEvent) {
        const text = event.clipboardData?.getData('text');
        if (text && (text.startsWith('http://') || text.startsWith('https://'))) {
            setTimeout(() => fetchInfo(), 0);
        }
    }
</script>

<section>
  <h2>Add New Download</h2>
  
  {#if error}
    <div class="message error">
      <strong>Error:</strong> {error}
    </div>
  {/if}

  <div class="form-group">
    <label for="url">Download URL</label>
    <div class="url-input-group">
      <input
        type="text" id="url" bind:value={url} class="url-input"
        placeholder="Enter or paste download URL..."
        on:paste={handlePaste} on:blur={fetchInfo} disabled={isLoading}
      />
      <button on:click={fetchInfo} disabled={isLoading || !url} class="browse-btn" >
        {#if isLoading && !downloadInfo}
          <div class="spinner"></div>
        {:else}
          Fetch Info
        {/if}
      </button>
    </div>
  </div>

  {#if downloadInfo}
    <div class="info-box">
      <h3 class="info-header">File Details</h3>
      <div class="details-grid">
        <span>File Name:</span><strong>{downloadInfo.fileName}</strong>
        <span>File Type:</span><strong>{downloadInfo.fileType}</strong>
        <span>Size:</span><strong>{formatBytes(downloadInfo.totalSize)}</strong>
      </div>
      
      <div class="form-group">
        <label for="save-path">Save To</label>
        <div class="path-selector">
          <input type="text" id="save-path" bind:value={customPath} class="path-input" placeholder="Default: {defaultDownloadFolder}" />
          <button on:click={chooseFolder} class="browse-btn">...</button>
        </div>
      </div>

      <button on:click={handleAddDownload} disabled={isLoading} class="download-btn" >
        {#if isLoading}
            <div class="spinner"></div>
        {:else}
            Start Download
        {/if}
      </button>
    </div>
  {/if}
</section>

<!-- STYLES -->
<style>
  section { max-width: 600px; margin: 0 auto; }
  .form-group { margin-bottom: 1.5rem; }
  label { display: block; margin-bottom: 0.5rem; font-weight: 500; }
  .url-input-group, .path-selector { display: flex; gap: 8px; }
  .url-input, .path-input {
    flex: 1; padding: 10px; background: #2a2a2a;
    border: 1px solid #444; border-radius: 4px;
    color: #fff; font-size: 14px;
  }
  .browse-btn {
    padding: 10px 20px; background: #555; border: none;
    border-radius: 4px; color: #fff; cursor: pointer;
    transition: background 0.3s; display: flex; align-items: center; justify-content: center;
    min-width: 80px;
  }
  .browse-btn:hover:not(:disabled) { background: #666; }
  .browse-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .download-btn {
    width: 100%; padding: 12px; background: #4CAF50;
    border: none; border-radius: 4px; color: white;
    font-size: 16px; cursor: pointer; transition: background 0.3s;
    display: flex; align-items: center; justify-content: center;
  }
  .download-btn:hover:not(:disabled) { background: #45a049; }
  .download-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .message {
    margin-bottom: 1rem; padding: 12px; border-radius: 4px;
    text-align: center;
  }
  .message.error { background: #f44336; color: white; }
  .spinner {
    border: 2px solid #f3f3f3; border-top: 2px solid #555;
    border-radius: 50%; width: 16px; height: 16px;
    animation: spin 1s linear infinite;
  }
  @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
  .info-box {
    background: #2a2a2a; padding: 1rem 1.5rem;
    border: 1px solid #444; border-radius: 8px; margin-top: 1rem;
  }
  .info-header { margin-top: 0; border-bottom: 1px solid #444; padding-bottom: 0.5rem; margin-bottom: 1rem; }
  .details-grid {
    display: grid; grid-template-columns: auto 1fr;
    gap: 0.5rem 1rem; align-items: center;
    margin-bottom: 1.5rem;
  }
  .details-grid strong { font-weight: 500; word-break: break-all; }
</style>
