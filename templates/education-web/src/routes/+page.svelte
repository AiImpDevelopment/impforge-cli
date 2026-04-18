// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
<script lang="ts">
  let count = $state(0);
  const label = $derived(count === 1 ? "click" : "clicks");
</script>

<h1>Welcome to education-web</h1>
<button onclick={() => count++}>
  {count} {label}
</button>
