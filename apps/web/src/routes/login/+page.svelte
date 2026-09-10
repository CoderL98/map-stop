<script lang="ts">
	import { api, setSession } from '$lib/api';

	let login = $state('admin');
	let password = $state('admin123');
	let error = $state('');
	let loading = $state(false);

	async function submit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const res = await api<{ token: string; user: { id: string; username: string; email?: string; role: string } }>(
				'/auth/login',
				{ method: 'POST', json: { login, password } }
			);
			setSession(res.token, res.user);
			window.location.href = '/plan';
		} catch (err) {
			error = err instanceof Error ? err.message : '登录失败';
		} finally {
			loading = false;
		}
	}
</script>

<div class="card" style="max-width:420px;margin:2rem auto;">
	<h1>登录</h1>
	<p style="color:#64748b;font-size:0.9rem;">支持用户名或邮箱。种子管理员默认 admin / admin123（可用环境变量覆盖）。</p>
	<form class="stack" onsubmit={submit}>
		<div>
			<label for="login">用户名或邮箱</label>
			<input id="login" bind:value={login} required />
		</div>
		<div>
			<label for="password">密码</label>
			<input id="password" type="password" bind:value={password} required />
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		<button type="submit" disabled={loading}>{loading ? '登录中…' : '登录'}</button>
	</form>
	<p style="margin-top:1rem;">没有账号？<a href="/register">注册</a></p>
</div>
