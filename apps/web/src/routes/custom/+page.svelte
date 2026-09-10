<script lang="ts">
	import { onMount } from 'svelte';
	import { api, getToken } from '$lib/api';
	import type { CustomPoint } from '$lib/types';

	let points = $state<CustomPoint[]>([]);
	let name = $state('');
	let lat = $state(30.26);
	let lon = $state(120.15);
	let radius_m = $state(80);
	let note = $state('');
	let error = $state('');
	let msg = $state('');

	onMount(async () => {
		if (!getToken()) {
			window.location.href = '/login';
			return;
		}
		await load();
	});

	async function load() {
		points = await api<CustomPoint[]>('/custom-points');
	}

	async function add(e: Event) {
		e.preventDefault();
		error = '';
		msg = '';
		try {
			await api('/custom-points', {
				method: 'POST',
				json: { name, lat, lon, radius_m, note: note || null, selected: true }
			});
			msg = '已添加';
			name = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function remove(id: string) {
		await api(`/custom-points/${id}`, { method: 'DELETE' });
		await load();
	}

	async function toggle(p: CustomPoint) {
		await api(`/custom-points/${p.id}`, {
			method: 'PUT',
			json: {
				name: p.name,
				lat: p.lat,
				lon: p.lon,
				radius_m: p.radius_m,
				note: p.note,
				selected: !p.selected
			}
		});
		await load();
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">自定义躲避点</h1>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;">
					<label>名称</label>
					<input bind:value={name} required />
				</div>
				<div style="width:120px;">
					<label>纬度</label>
					<input type="number" step="any" bind:value={lat} required />
				</div>
				<div style="width:120px;">
					<label>经度</label>
					<input type="number" step="any" bind:value={lon} required />
				</div>
				<div style="width:100px;">
					<label>半径米</label>
					<input type="number" bind:value={radius_m} min="1" required />
				</div>
			</div>
			<div>
				<label>备注</label>
				<input bind:value={note} />
			</div>
			{#if error}<p class="err">{error}</p>{/if}
			{#if msg}<p class="ok">{msg}</p>{/if}
			<button type="submit">添加</button>
		</form>
	</div>

	<div class="card">
		<table>
			<thead>
				<tr><th>规划时生效</th><th>名称</th><th>坐标</th><th>半径</th><th></th></tr>
			</thead>
			<tbody>
				{#each points as p}
					<tr>
						<td><input type="checkbox" checked={p.selected} onchange={() => toggle(p)} /></td>
						<td>{p.name}</td>
						<td>{p.lat.toFixed(5)}, {p.lon.toFixed(5)}</td>
						<td>{p.radius_m} m</td>
						<td><button class="danger" type="button" onclick={() => remove(p.id)}>删除</button></td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
