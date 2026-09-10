<script lang="ts">
	import { onMount } from 'svelte';
	import { api, getToken, getUser } from '$lib/api';
	import type { SystemPoint } from '$lib/types';

	let points = $state<SystemPoint[]>([]);
	let name = $state('');
	let lat = $state(30.27);
	let lon = $state(120.16);
	let radius_m = $state(80);
	let note = $state('');
	let error = $state('');
	let msg = $state('');
	let fileInput: HTMLInputElement;

	onMount(async () => {
		if (!getToken() || getUser()?.role !== 'admin') {
			window.location.href = '/login';
			return;
		}
		await load();
	});

	async function load() {
		points = await api<SystemPoint[]>('/system-points');
	}

	async function add(e: Event) {
		e.preventDefault();
		error = '';
		msg = '';
		try {
			await api('/system-points', {
				method: 'POST',
				json: { name, lat, lon, radius_m, note: note || null, enabled: true }
			});
			msg = '已创建';
			name = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function toggle(p: SystemPoint) {
		await api(`/system-points/${p.id}/enabled`, {
			method: 'PATCH',
			json: { enabled: !p.enabled }
		});
		await load();
	}

	async function remove(id: string) {
		if (!confirm('确认删除该系统点？')) return;
		await api(`/system-points/${id}`, { method: 'DELETE' });
		await load();
	}

	async function importCsv() {
		error = '';
		msg = '';
		const file = fileInput?.files?.[0];
		if (!file) {
			error = '请选择 CSV';
			return;
		}
		const fd = new FormData();
		fd.append('file', file);
		const token = getToken();
		const res = await fetch('/api/system-points/import', {
			method: 'POST',
			headers: token ? { Authorization: `Bearer ${token}` } : {},
			body: fd
		});
		const data = await res.json();
		if (!res.ok) {
			error = data.error || '导入失败';
			return;
		}
		msg = `导入 ${data.imported} 条` + (data.errors?.length ? `，错误 ${data.errors.length} 条` : '');
		await load();
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">系统躲避点管理</h1>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;"><label>名称</label><input bind:value={name} required /></div>
				<div style="width:120px;"><label>纬度</label><input type="number" step="any" bind:value={lat} required /></div>
				<div style="width:120px;"><label>经度</label><input type="number" step="any" bind:value={lon} required /></div>
				<div style="width:100px;"><label>半径米</label><input type="number" bind:value={radius_m} required /></div>
			</div>
			<div><label>备注</label><input bind:value={note} /></div>
			<button type="submit">新增系统点</button>
		</form>
		<div class="row" style="margin-top:1rem;">
			<input type="file" accept=".csv,text/csv" bind:this={fileInput} />
			<button type="button" class="secondary" onclick={importCsv}>CSV 导入</button>
			<a href="/docs/templates/avoid-points.csv" style="display:none;">template</a>
			<span style="font-size:0.85rem;color:#64748b;">模板见仓库 docs/templates/avoid-points.csv</span>
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		{#if msg}<p class="ok">{msg}</p>{/if}
	</div>

	<div class="card">
		<table>
			<thead>
				<tr><th>启用</th><th>名称</th><th>坐标</th><th>半径</th><th></th></tr>
			</thead>
			<tbody>
				{#each points as p}
					<tr>
						<td><input type="checkbox" checked={p.enabled} onchange={() => toggle(p)} /></td>
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
