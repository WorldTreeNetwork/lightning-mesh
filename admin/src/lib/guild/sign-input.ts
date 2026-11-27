import type { GuildSignInput, GuildVerb } from './api';

/**
 * Body for `sign_guild_op`. The command reads this computer's signing key
 * when the seed is empty. Join starts a house, so it carries no guild id.
 * Change and nest use the selected guild. Nothing here is a network name.
 */
export function guildSignInput(verb: GuildVerb, guildIdHex?: string): GuildSignInput {
	const guild = guildIdHex?.trim();
	return {
		verb,
		actorSeedHex: '',
		guildIdHex: verb === 'join' || !guild ? undefined : guild,
		nodeIdHex: undefined
	};
}
