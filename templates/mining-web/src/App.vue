// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
<script setup lang="ts">
import { ref } from "vue";
const count = ref(0);
</script>

<template>
  <h1>Welcome to mining-web</h1>
  <button @click="count++">{{ count }}</button>
</template>
