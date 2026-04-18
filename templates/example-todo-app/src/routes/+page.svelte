<script lang="ts">
  type Todo = { id: number; text: string; done: boolean };
  let todos = $state<Todo[]>([]);
  let next_id = $state(1);
  let draft = $state("");

  function add() {
    const text = draft.trim();
    if (text === "") return;
    todos.push({ id: next_id, text, done: false });
    next_id++;
    draft = "";
  }

  function toggle(id: number) {
    const t = todos.find((t) => t.id === id);
    if (t !== undefined) t.done = !t.done;
  }

  function remove(id: number) {
    const idx = todos.findIndex((t) => t.id === id);
    if (idx >= 0) todos.splice(idx, 1);
  }

  const remaining = $derived(todos.filter((t) => !t.done).length);
</script>

<main class="min-h-screen bg-zinc-950 text-zinc-100 p-8">
  <div class="max-w-md mx-auto">
    <h1 class="text-3xl font-bold mb-6">Example Todo</h1>
    <p class="text-zinc-400 text-sm mb-4">
      Reference template — built with the
      <a class="underline" href="https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/template.json.v1.md">
        template.json v1 spec
      </a>.
    </p>

    <form onsubmit={(e) => { e.preventDefault(); add(); }} class="flex gap-2 mb-6">
      <input
        bind:value={draft}
        placeholder="Add a todo…"
        class="flex-1 bg-zinc-900 border border-zinc-700 rounded px-3 py-2"
      />
      <button class="bg-emerald-500 hover:bg-emerald-400 text-zinc-950 font-medium px-4 rounded">
        Add
      </button>
    </form>

    <ul class="space-y-2">
      {#each todos as t (t.id)}
        <li class="flex items-center gap-3 bg-zinc-900 rounded px-3 py-2">
          <input
            type="checkbox"
            checked={t.done}
            onchange={() => toggle(t.id)}
            class="w-4 h-4"
          />
          <span class="flex-1" class:line-through={t.done}>{t.text}</span>
          <button onclick={() => remove(t.id)} class="text-zinc-500 hover:text-rose-400">
            ×
          </button>
        </li>
      {/each}
    </ul>

    {#if todos.length > 0}
      <p class="text-zinc-500 text-sm mt-4">{remaining} remaining</p>
    {/if}
  </div>
</main>
