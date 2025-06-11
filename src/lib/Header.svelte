<!-- src/lib/Header.svelte -->
<header>
  <h1>VeloDown</h1>
  <nav>
    <a href="/">Downloads</a>
    <a href="/add">Add New</a>
    <a href="/settings">Settings</a>
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
  a {
    margin-left: 15px;
    color: #f0f0f0;
    text-decoration: none;
  }
  a:hover {
    text-decoration: underline;
  }
</style>
