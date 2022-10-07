<script lang="ts">
    import Sticky from "./Sticky.svelte";
    import UserIndicator from "./UserIndicator.svelte";
</script>
<Sticky>
    <nav id="navbar">
        <img class="lapce" src="/lapce-35px.png" alt="Lapce Icon">
        <h1 class="title">Lapce Registry</h1>
        <div class="spacer"></div>
        <UserIndicator/>
    </nav>
</Sticky>
<style>
    .lapce {
        margin-right: 15px;
    }
    .title {
        font-weight: 200;
    }
    #navbar {
        font-family: "Poppins", Arial, Helvetica, sans-serif;
        display: flex;
        padding: 20px 120px;
        align-items: center;
        color: rgb(42 51 66);
    }
    .spacer {
        flex-grow: 1;
    }
</style>