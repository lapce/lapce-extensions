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
        fetch("/api/user")
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
</script>

{#if logged_in}
<div class="indicator">
        <!-- svelte-ignore a11y-img-redundant-alt -->
        <img
            alt="profile picture"
            class="pf"
            width="30"
            src={user.avatar_url}
        />
        <span id="username">{user.username}</span>
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
    #username {
        font-weight: bold;
    }
    .indicator {
        display: flex;
        justify-items: center;
        align-items: center;
    }
    .pf {
        border-radius: 50%;
        margin-right: 10px;
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
        background: rgb(17, 17, 107);
    }
    #gh-icon {
        margin-right: 5px;
    }
</style>
