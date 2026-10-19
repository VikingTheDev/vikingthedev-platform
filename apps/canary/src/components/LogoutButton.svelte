<script lang="ts">
  import createClient from "openapi-fetch";
  import type { paths } from "@vikingthedev/api-types";

  const api = createClient<paths>();
  let hasFailed = $state(false);
  let isPending = $state(false);

  async function requestLogout(): Promise<boolean> {
    try {
      const { response } = await api.POST("/api/auth/logout", { body: {} });
      return response.ok;
    } catch {
      return false;
    }
  }

  async function logOut() {
    hasFailed = false;
    isPending = true;
    if (await requestLogout()) {
      location.reload();
      return;
    }
    isPending = false;
    hasFailed = true;
  }
</script>

<button type="button" onclick={logOut} disabled={isPending}>Log out</button>
{#if hasFailed}
  <p role="alert">Logging out failed. Please try again.</p>
{/if}
