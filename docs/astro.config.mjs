// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://kylebastien.github.io',
	base: '/sextant-mcp',
	trailingSlash: 'ignore',
	integrations: [
		starlight({
			title: 'Sextant',
			description: 'Code-quality grader for AI-agent workflows.',
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/kylebastien/sextant-mcp',
				},
			],
			editLink: {
				baseUrl: 'https://github.com/kylebastien/sextant-mcp/edit/main/docs/',
			},
			lastUpdated: true,
			sidebar: [
				{
					label: 'Getting Started',
					items: [{ autogenerate: { directory: 'getting-started' } }],
				},
				{
					label: 'Concepts',
					items: [{ autogenerate: { directory: 'concepts' } }],
				},
				{
					label: 'CLI',
					items: [{ autogenerate: { directory: 'cli' } }],
				},
				{
					label: 'MCP Server',
					items: [{ autogenerate: { directory: 'mcp' } }],
				},
				{
					label: 'GitHub Action',
					items: [{ autogenerate: { directory: 'action' } }],
				},
				{
					label: 'Claude Code Plugin',
					items: [{ autogenerate: { directory: 'plugin' } }],
				},
				{
					label: 'Rules Catalog',
					items: [{ autogenerate: { directory: 'rules' } }],
				},
				{
					label: 'Configuration',
					items: [{ autogenerate: { directory: 'configuration' } }],
				},
				{
					label: 'Architecture',
					items: [{ autogenerate: { directory: 'architecture' } }],
				},
			],
			customCss: ['./src/styles/custom.css'],
		}),
	],
});
