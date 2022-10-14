<script lang="ts">
  import { parse as parseCookies } from "cookie";
  interface User {
    name: string;
    username: string;
    id: number;
    avatar_url: string;
  }
  const cookies = parseCookies(document.cookie);
  let user = null;
  let logged_in = false;
  if (cookies.token) {
    fetch("/api/v1/user")
      .then((res) => {
        if (res.status == 200) {
          return res.json() as Promise<User>;
        }
        return null;
      })
      .then((f_user) => {
        user = f_user;
        if (user) logged_in = true;
      });
  }
  function logout() {
    fetch("/api/v1/session", {
      method: "DELETE",
    }).then(() => window.location.reload());
  }
</script>

{#if logged_in}
  <div class="indicator">
    <img alt="pfp" class="pf" width="30" src={user.avatar_url} />
    <span id="username">{user.name}</span>
    <button id="logout" on:click={logout}>Logout</button>
  </div>
{:else}
  <a class="login-button" href="/login/github"
    ><img
      id="gh-icon"
      alt="github white icon"
      width="12"
      src="/GitHub-Mark-Light-64px.png"
    />Login</a
  >
{/if}

<style>
  #logout {
    padding: 5px 10px;
    background-color: rgb(224, 97, 97);
    border-radius: 3px;
    border: 1px black solid;
    cursor: pointer;
  }
  #logout:active {
    background-color: rgb(121, 54, 54);
    color: white;
  }
  #username {
    font-weight: bold;
    margin: 0px 10px;
  }
  .indicator {
    display: flex;
    justify-items: center;
    align-items: center;
  }
  .pf {
    border-radius: 50%;
    border: rgb(26, 141, 161) 2px solid;
  }
  .login-button {
    background: rgb(22, 22, 22);
    padding: 10px;
    color: white;
    font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
    font-weight: 700;
    border-radius: 5px;
    text-decoration: none;
  }
  .login-button:hover {
    background: rgb(31, 31, 31);
  }

  .login-button:active {
    background: rgb(0, 0, 0);
  }
  #gh-icon {
    margin-right: 5px;
  }
</style>
