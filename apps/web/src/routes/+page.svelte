<script lang="ts">
	import { onMount } from 'svelte';
	import { getUser } from '$lib/api';

	onMount(() => {
		const u = getUser();
		window.location.href = u ? '/plan' : '/login';
	});
</script>

<p>正在跳转…</p>
