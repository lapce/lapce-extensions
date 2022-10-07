<script lang="ts">
	import {onMount, onDestroy} from 'svelte'
	let show = false;
	let lastScrollY = 0;
	onMount(() => {
		window.addEventListener("scroll", () => {
			show = (window.scrollY > lastScrollY);
			lastScrollY = window.scrollY;
		});
	})
	
	onDestroy(() => {
		window.onscroll = () => {}
	})
</script>

<style>

	.scrolled {
		transform: translate(0,calc(-100% - 1rem))
	}
	.sticky {
		width: 100%;
		position: fixed;
		padding: 10px;
        background: linear-gradient(to right top, #6ca0e0, #63a9e6, #5ab2eb, #2eb9e7, #00bfdd, #00c3cd, #10c6ba);
		transition: 1s ease;
	}
</style>

<div class="sticky" class:scrolled={show}>
	<slot/>
</div>
