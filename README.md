# Lapce Registry
![Heroku deploy](https://img.shields.io/github/deployments/lapce/lapce-extensions/lapce-extensions?color=%236762A6&label=heroku%20deploy)

This is the software running the [lapce plugin registry](https://registry.lapce.dev), this manages and hosts plugins that the community uploads.

## Run the registry locally
### Requirements:
- Docker Compose (to run the databases)
- Rust Nightly or [rustup](https://rustup.rs)
- Node.js
- Nodemon: `npm install -g nodemon`
### Running
1. First create a .env from .env.example: `cp .env.example .env`
2. Create a GitHub OAuth app (always configure the redirect URL to `http://localhost:8000/auth/github`)
3. Configure the GitHub Single Sign-On:
```
GH_CLIENT_ID=<appid>
GH_CLIENT_SECRET=<appsecret>
GH_REDIRECT_URL=http://localhost:8000/auth/github
```
4. `npm install` to install the JS dependencies
5. `make client` to build the frontend and watch for changes
6. Run `make` on a separate terminal to build 
and run the backend and watch for changes

And now you've got a dev environment :tada:!
