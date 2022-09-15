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
                if(res.status == 200){
                    logged_in = true;
                    return null;
                }
                return res.json() as Promise<User>
            })
            .then((f_user) => {
                user = f_user;
            });
    }
</script>

{#if logged_in}
    {user.name}
{:else}
    <a class="login-button" href="/login/github"><img id="gh-icon" alt="github white icon" width="12" src="/GitHub-Mark-Light-64px.png"/>Login</a>
{/if}

<style>
    @import url('https://fonts.googleapis.com/css2?family=Roboto:wght@700&display=swap');
    .login-button {
        background: rgb(77, 77, 77);
        background: linear-gradient(
            180deg,
            rgb(77, 77, 77) 0%,
            rgba(22, 22, 22, 1) 100%
        );
        padding: 10px;
        color: white;
        font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
        font-weight: 700;
        border-radius: 5px;
        text-decoration: none;

    }
    #gh-icon {
        padding-right: 5px;
    }
</style>
