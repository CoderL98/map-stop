<script lang="ts">
	import { onMount } from 'svelte';
	import { getUser } from '$lib/api';

	let loggedIn = $state(false);

	onMount(() => {
		loggedIn = !!getUser();
	});
</script>

<div class="hero card">
	<h1>map-stop 躲避规划</h1>
	<p class="lead">
		系统维护躲避点库，规划时<strong>硬避开</strong>所有生效圆形禁区（绝对不穿行）；也可添加自定义点，或上传候选经管理员审核后升为系统点。
	</p>
	<ul class="feats">
		<li>算路可插拔：默认高德 Web 服务（有 Key），否则杭州演示网格 A*</li>
		<li>底图：有 <code>AMAP_JS_KEY</code> 时用<strong>高德 JS API 2.0</strong>（GCJ-02）；否则 Leaflet + OSM</li>
		<li>地名搜索：高德输入提示/地理编码，或 Nominatim（embedded）</li>
		<li>自定义点勾选、半径可调；上传审核入库</li>
	</ul>
	<div class="row cta">
		{#if loggedIn}
			<a class="btn" href="/plan">开始规划路线</a>
			<a class="btn secondary" href="/custom">管理自定义点</a>
		{:else}
			<a class="btn" href="/login">登录开始</a>
			<a class="btn secondary" href="/register">注册账号</a>
		{/if}
	</div>
	<p class="foot">
		v1 使用高德须遵守其开放平台条款与配额；无 Key 时回退 OSM / Leaflet（© OpenStreetMap contributors，ODbL）。未使用百度 / 腾讯 SDK。
	</p>
</div>

<style>
	.hero {
		max-width: 720px;
		margin: 2rem auto;
		padding: 2rem 1.75rem;
	}
	.hero h1 {
		margin: 0 0 0.75rem;
		font-size: 1.75rem;
	}
	.lead {
		color: #334155;
		font-size: 1.05rem;
	}
	.feats {
		margin: 1rem 0 1.25rem;
		padding-left: 1.25rem;
		color: #475569;
	}
	.cta a.btn {
		display: inline-block;
		text-decoration: none;
	}
	.cta a.btn.secondary {
		background: #64748b;
		color: #fff;
	}
	.foot {
		margin: 1.25rem 0 0;
		font-size: 0.85rem;
		color: #94a3b8;
	}
</style>
