<script lang="ts">
	import { Badge } from "$lib/components/ui/badge";
	import type { GuildRole } from "./mock/types";

	let { role }: { role: GuildRole | "admin" | "member" } = $props();

	const tone: Record<string, string> = {
		admin: "border-transparent bg-[var(--accent)] text-[var(--accent-ink)]",
		member: "border-transparent bg-[var(--surface-2)] text-[var(--text)]",
		absent: "bg-transparent text-[var(--warn)] border-[var(--warn)]",
		"no-seed": "bg-transparent text-[var(--bad)] border-[var(--bad)]",
	};

	const label: Record<string, string> = {
		admin: "admin",
		member: "member",
		absent: "absent",
		"no-seed": "no seed",
	};
</script>

<Badge variant="outline" class={tone[role]}>{label[role] ?? role}</Badge>
