<script lang="ts">
  import Header from './lib/Header.svelte';
  import DownloadList from './lib/DownloadList.svelte';
  import AddDownload from './lib/AddDownload.svelte';
  import Settings from './lib/Settings.svelte';

  // This variable controls which view is currently visible.
  let currentView: 'downloads' | 'add' | 'settings' = 'downloads';

  // This function will be called by the Header component to change the view.
  function navigate(event: CustomEvent) {
    currentView = event.detail.view;
  }
</script>

<main class="container">
  <Header on:navigate={navigate} />

  {#if currentView === 'downloads'}
    <DownloadList />
  {:else if currentView === 'add'}
    <AddDownload />
  {:else if currentView === 'settings'}
    <Settings />
  {/if}
</main>

<style>
  .container {
    margin: 0 auto;
    padding: 2rem;
    max-width: 800px;
    font-family: sans-serif;
    color: #f0f0f0;
  }
</style>
