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
	let savingId = $state<string | null>(null);

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
		error = '';
		msg = '';
		try {
			await api(`/custom-points/${id}`, { method: 'DELETE' });
			msg = '已删除';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '删除失败';
		}
	}

	async function toggle(p: CustomPoint) {
		error = '';
		try {
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
		} catch (err) {
			error = err instanceof Error ? err.message : '更新失败';
		}
	}

	async function saveRadius(p: CustomPoint, raw: string | number) {
		const next = typeof raw === 'number' ? raw : Number(raw);
		if (!Number.isFinite(next) || next <= 0 || next > 50000) {
			error = '半径必须大于 0 且不超过 50000 米';
			await load();
			return;
		}
		if (next === p.radius_m) return;
		error = '';
		msg = '';
		savingId = p.id;
		try {
			await api(`/custom-points/${p.id}`, {
				method: 'PUT',
				json: {
					name: p.name,
					lat: p.lat,
					lon: p.lon,
					radius_m: next,
					note: p.note,
					selected: p.selected
				}
			});
			msg = `已更新「${p.name}」半径为 ${next} m`;
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '更新失败';
			await load();
		} finally {
			savingId = null;
		}
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">自定义躲避点</h1>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;">
					<label for="c-name">名称</label>
					<input id="c-name" bind:value={name} required />
				</div>
				<div style="width:120px;">
					<label for="c-lat">纬度</label>
					<input id="c-lat" type="number" step="any" bind:value={lat} required />
				</div>
				<div style="width:120px;">
					<label for="c-lon">经度</label>
					<input id="c-lon" type="number" step="any" bind:value={lon} required />
				</div>
				<div style="width:100px;">
					<label for="c-r">半径米</label>
					<input id="c-r" type="number" bind:value={radius_m} min="1" required />
				</div>
			</div>
			<div>
				<label for="c-note">备注</label>
				<input id="c-note" bind:value={note} />
			</div>
			{#if error}<p class="err">{error}</p>{/if}
			{#if msg}<p class="ok">{msg}</p>{/if}
			<button type="submit">添加</button>
		</form>
	</div>

	<div class="card">
		<p class="muted" style="margin-top:0;">可在列表中直接修改半径；勾选表示参与规划。</p>
		<table>
			<thead>
				<tr><th>规划时生效</th><th>名称</th><th>坐标</th><th>半径(m)</th><th></th></tr>
			</thead>
			<tbody>
				{#each points as p}
					<tr>
						<td><input type="checkbox" checked={p.selected} onchange={() => toggle(p)} /></td>
						<td>{p.name}</td>
						<td>{p.lat.toFixed(5)}, {p.lon.toFixed(5)}</td>
						<td style="width:110px;">
							<input
								type="number"
								min="1"
								max="50000"
								value={p.radius_m}
								disabled={savingId === p.id}
								style="width:90px;"
								onchange={(e) => saveRadius(p, (e.currentTarget as HTMLInputElement).value)}
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.preventDefault();
										saveRadius(p, (e.currentTarget as HTMLInputElement).value);
									}
								}}
							/>
						</td>
						<td><button class="danger" type="button" onclick={() => remove(p.id)}>删除</button></td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if points.length === 0}
			<p class="muted">暂无自定义点</p>
		{/if}
	</div>
</div>
