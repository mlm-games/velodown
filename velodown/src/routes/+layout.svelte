<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import Header from '../lib/Header.svelte';
</script>

<main class="container">
  <Header />

  <!-- SvelteKit will render the current page's content here -->
  <slot />
</main>

<style>
  .container {
    margin: 0 auto;
    padding: 2rem;
    max-width: 800px;
    font-family: sans-serif;
    color: #f0f0f0;
    background-color: #242424;
    min-height: 100vh;
  }
</style>
