import { signGuildOp, type GuildReceipt, type GuildVerb } from './api';

/** Operator copy. Protocol kinds stay keyspace.* in the receipt. */
export const GUILD_LABEL = 'Guild';

const HEX32 = /^[0-9a-fA-F]{64}$/;

export class GuildStore {
	actorSeed = $state('');
	guildId = $state('');
	nodeId = $state('');
	signing = $state(false);
	error = $state<string | null>(null);
	receipt = $state<GuildReceipt | null>(null);

	get seedOk(): boolean {
		return HEX32.test(this.actorSeed.trim());
	}

	get guildOk(): boolean {
		return HEX32.test(this.guildId.trim());
	}

	get nodeOk(): boolean {
		return HEX32.test(this.nodeId.trim());
	}

	can(verb: GuildVerb): boolean {
		if (this.signing || !this.seedOk) return false;
		if (verb === 'join') return true;
		if (verb === 'assign-node') return this.guildOk && this.nodeOk;
		return this.guildOk;
	}

	async sign(verb: GuildVerb) {
		if (!this.can(verb)) return;
		this.signing = true;
		this.error = null;
		this.receipt = null;
		try {
			this.receipt = await signGuildOp({
				verb,
				actorSeedHex: this.actorSeed.trim(),
				guildIdHex: this.guildId.trim() || undefined,
				nodeIdHex: this.nodeId.trim() || undefined
			});
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		} finally {
			this.signing = false;
		}
	}
}

export const guild = new GuildStore();
