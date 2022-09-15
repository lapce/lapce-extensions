<script lang="ts">
  interface User {
    name: string,
    username: string,
    id: number,
    avatar_url: string
  }
  import {parse as parseCookies} from "cookie"
  const cookies = parseCookies(document.cookie);
  let user = null;
  fetch("/api/user").then(res => (res.json() as Promise<User>)).then(f_user => user = f_user);
</script>
<main>
  {#if !cookies.token}
    <a href="/login/github">LOGIN WITH GITHUB</a>
  {:else}
    {#if user}
      Logged in as {user.name}
    {:else}
      Loading...
    {/if}
  {/if}
</main>