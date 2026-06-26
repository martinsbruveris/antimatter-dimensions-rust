<script setup>
import { credits } from "../data/credits";

const roles = credits.roles;

// People with the given role, sorted by name — mirrors the original
// CreditsDisplay.relevantPeople().
function relevantPeople(role) {
  return credits.people
    .filter((x) =>
      typeof x.roles === "number" ? x.roles === role : x.roles.includes(role)
    )
    .sort((a, b) => a.name.localeCompare(b.name));
}

// Minimal pluralize: the original `pluralize(word, count)` returns the
// singular for a count of 1, otherwise appends "s" — which is correct for
// every role title used here (Developer→Developers, Tester→Testers, …).
function pluralize(word, count) {
  return count === 1 ? word : `${word}s`;
}
</script>

<template>
  <div class="c-credits">
    <div
      v-for="role in roles.count"
      :key="role"
    >
      <h2 class="c-credits-section">
        {{ pluralize(roles[role], relevantPeople(role).length) }}
      </h2>
      <div :class="{ 'l-credits--bulk': relevantPeople(role).length > 10 }">
        <div
          v-for="person in relevantPeople(role)"
          :key="person.name"
          class="c-credit-entry"
        >
          {{ person.name }}
          <span v-if="person.name2">
            ({{ person.name2 }})
          </span>
        </div>
      </div>
    </div>

    <br><br><br><br><br><br><br><br><br>
    <h1 class="c-credits-header">
      Thank you so much for playing!
    </h1>
  </div>
</template>

<style scoped>
.c-credits {
  text-align: center;
}

/* `c-credits-header` (the "Thank you" line) is coloured by the vendored
   `.t-normal .c-credits-header` rule (red), matching the original. */

.c-credits-section {
  color: var(--color-text);
  text-shadow: 1px 1px 2px turquoise;
  font-size: 2rem;
  margin-top: 10rem;
  margin-bottom: 2rem;
}

.l-credits--bulk {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  width: 76%;
  position: relative;
  left: 12%;
}

.c-credit-entry {
  font-size: 1.3rem;
  margin-top: 1rem;
}
</style>
