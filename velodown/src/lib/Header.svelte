<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();

  function goTo(view: string) {
    dispatch('navigate', { view });
  }
</script>

<header>
  <h1>VeloDown</h1>
  <nav>
    <button on:click={() => goTo('downloads')}>Downloads</button>
    <button on:click={() => goTo('add')}>Add New</button>
    <button on:click={() => goTo('settings')}>Settings</button>
  </nav>
</header>

<style>
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid #444;
    padding-bottom: 1rem;
    margin-bottom: 2rem;
  }
  h1 {
    margin: 0;
  }
  button {
    margin-left: 10px;
  }
</style>
