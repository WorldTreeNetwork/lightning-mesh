<script lang="ts">
  let {
    guildName,
    onLocalName,
    onAssign,
  }: {
    guildName: string;
    onLocalName: (name: string) => void;
    onAssign: (nodeName: string, nodeId: string) => void;
  } = $props();

  const NODE_ID = /^[0-9a-fA-F]{64}$/;

  let localName = $state('');
  let nodeName = $state('');
  let nodeId = $state('');

  $effect(() => {
    localName = guildName;
  });

  const canSaveName = $derived(localName.trim().length > 0);
  const canAssign = $derived(nodeName.trim().length > 0 && NODE_ID.test(nodeId.trim()));

  function saveName() {
    const name = localName.trim();
    if (name) onLocalName(name);
  }

  function assign() {
    if (!canAssign) return;
    onAssign(nodeName.trim(), nodeId.trim());
    nodeName = '';
    nodeId = '';
  }
</script>

<div class="editors">
  <section>
    <h3>Local name</h3>
    <form onsubmit={(e) => { e.preventDefault(); saveName(); }}>
      <label>
        <span>Display name</span>
        <input type="text" autocomplete="off" spellcheck="false" bind:value={localName} />
      </label>
      <p class="helper">Saved on this computer only. The guild log has no rename.</p>
      <div class="actions">
        <button type="submit" disabled={!canSaveName}>Save name</button>
      </div>
    </form>
  </section>

  <section>
    <h3>Assign a node</h3>
    <form onsubmit={(e) => { e.preventDefault(); assign(); }}>
      <label>
        <span>Node name</span>
        <input type="text" autocomplete="off" spellcheck="false" bind:value={nodeName} />
      </label>
      <label>
        <span>Node id</span>
        <input
          class="mono"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="64 hex characters"
          aria-invalid={nodeId.trim().length > 0 && !NODE_ID.test(nodeId.trim())}
          bind:value={nodeId}
        />
      </label>
      <p class="helper">
        Records this router's existing node id on the guild, then opens an access-node guild.
        Mock only: nothing is signed.
      </p>
      <div class="actions">
        <button type="submit" disabled={!canAssign}>Assign</button>
      </div>
    </form>
  </section>

</div>

<style>
  .editors {
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-family: var(--font);
    color: var(--text);
  }

  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
  }

  h3 {
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 600;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }

  input {
    font: inherit;
    font-size: 13px;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 8px;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  input[aria-invalid='true'] {
    border-color: var(--bad);
  }

  input.mono {
    font-family: var(--mono);
  }

  .helper {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  button {
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    border-radius: var(--radius);
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--accent);
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

</style>
