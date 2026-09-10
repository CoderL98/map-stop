<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { getUser, clearSession, type User } from '$lib/api';
	import { onMount } from 'svelte';

	let { children } = $props();
	let user = $state<User | null>(null);

	onMount(() => {
		user = getUser();
	});

	function logout() {
		clearSession();
		user = null;
		window.location.href = '/login';
	}

	function navClass(path: string) {
		return $page.url.pathname === path || $page.url.pathname.startsWith(path + '/')
			? 'active'
			: '';
	}
</script>

<nav class="nav">
	<a class="brand" href="/">map-stop 躲避规划</a>
	{#if user}
		<a class={navClass('/plan')} href="/plan">路线规划</a>
		<a class={navClass('/custom')} href="/custom">自定义点</a>
		<a class={navClass('/uploads')} href="/uploads">我的上传</a>
		{#if user.role === 'admin'}
			<a class={navClass('/admin/system')} href="/admin/system">系统点</a>
			<a class={navClass('/admin/review')} href="/admin/review">审核</a>
		{/if}
		<span style="opacity:0.8">{user.username}（{user.role === 'admin' ? '管理员' : '用户'}）</span>
		<button class="secondary" type="button" onclick={logout}>退出</button>
	{:else}
		<a class={navClass('/login')} href="/login">登录</a>
		<a class={navClass('/register')} href="/register">注册</a>
	{/if}
</nav>

<main class="layout">
	{@render children()}
</main>
