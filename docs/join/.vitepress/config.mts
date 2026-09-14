// Human site for the join guide. The same Markdown is the agent-readable
// source (see ../llms.txt). Build: bunx vitepress build docs/join
export default {
	title: 'Join Lightning Mesh',
	description: 'Step-by-step guide to joining a Lightning Mesh network.',
	cleanUrls: true,
	lastUpdated: false,
	srcExclude: ['llms.txt'],
	themeConfig: {
		nav: [
			{ text: 'Phone or laptop', link: '/person/01-connect' },
			{ text: 'Bring a router', link: '/node/01-hardware-and-flash' },
			{ text: 'Publish a service', link: '/publish/01-publish-a-service' }
		],
		sidebar: [
			{ text: 'Start here', items: [{ text: 'Overview', link: '/' }] },
			{
				text: 'Join with a phone or laptop',
				items: [
					{ text: '1. Connect to the Wi-Fi', link: '/person/01-connect' },
					{ text: '2. Say hello', link: '/person/02-hello-mesh' },
					{ text: '3. Create your identity', link: '/person/03-identity' },
					{ text: '4. Sign in to mesh apps', link: '/person/04-sign-in' },
					{ text: '5. Find services', link: '/person/05-services' },
					{ text: '6. Take your identity with you', link: '/person/06-leave' },
					{ text: 'Troubleshooting', link: '/person/troubleshooting' }
				]
			},
			{
				text: 'Bring a router',
				items: [
					{ text: '1. Hardware and OpenWrt', link: '/node/01-hardware-and-flash' },
					{ text: '2. Install', link: '/node/02-install' },
					{ text: '3. Join the mesh', link: '/node/03-join-the-mesh' },
					{ text: '4. Operate', link: '/node/04-operate' }
				]
			},
			{
				text: 'Publish a service',
				items: [
					{ text: '1. Publish a service', link: '/publish/01-publish-a-service' },
					{ text: '2. Mini-apps (coming soon)', link: '/publish/02-mini-apps' }
				]
			},
			{ text: 'Contribute', items: [{ text: 'Contribute', link: '/contribute' }] }
		],
		outline: 'deep'
	}
};
