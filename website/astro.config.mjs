// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';

const jsonLd = {
	'@context': 'https://schema.org',
	'@type': 'SoftwareApplication',
	name: 'dusrelpdf',
	applicationCategory: 'DeveloperApplication',
	operatingSystem: 'Cross-platform',
	description:
		'Offline PDF toolkit — merge, split, extract, remove, reorder, rotate, images-to-PDF, watermark, page numbers and signature-image placement. One Rust core, zero network calls.',
	author: { '@type': 'Person', name: 'dusmamud' },
	offers: { '@type': 'Offer', price: '0', priceCurrency: 'INR' },
	softwareVersion: '0.1.0',
	url: 'https://dusrelpdf.pages.dev/',
	downloadUrl: 'https://www.npmjs.com/package/dusrelpdf',
	programmingLanguage: ['Rust', 'JavaScript', 'TypeScript', 'Python'],
	license: 'https://github.com/dusmamud/dusrelpdf/blob/main/LICENSE',
};

// https://astro.build/config
export default defineConfig({
	// Placeholder until the site is deployed (Cloudflare Pages, like
	// indycheck). Update this when a real domain is attached.
	site: 'https://dusrelpdf.pages.dev',
	integrations: [
		starlight({
			title: 'dusrelpdf',
			description:
				'Offline PDF toolkit — merge, split, extract, remove, reorder, rotate, images-to-PDF, watermark, page numbers and signature placement. Rust core, zero network calls.',
			logo: {
				src: './src/assets/logo.svg',
				alt: 'dusrelpdf logo',
			},
			favicon: '/favicon.svg',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/dusmamud/dusrelpdf' },
			],
			head: [
				{ tag: 'meta', attrs: { property: 'og:type', content: 'website' } },
				{ tag: 'meta', attrs: { property: 'og:site_name', content: 'dusrelpdf' } },
				{ tag: 'meta', attrs: { property: 'og:title', content: 'dusrelpdf — Offline PDF toolkit' } },
				{
					tag: 'meta',
					attrs: {
						property: 'og:description',
						content:
							'Merge, split, extract, rotate, watermark and number PDF pages offline. One Rust core, bindings for JavaScript, Python and Rust, plus a CLI.',
					},
				},
				{ tag: 'meta', attrs: { property: 'og:url', content: 'https://dusrelpdf.pages.dev/' } },
				{ tag: 'meta', attrs: { property: 'og:image', content: 'https://dusrelpdf.pages.dev/og.svg' } },
				{ tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
				{ tag: 'meta', attrs: { name: 'twitter:title', content: 'dusrelpdf — Offline PDF toolkit' } },
				{
					tag: 'meta',
					attrs: {
						name: 'twitter:description',
						content:
							'Merge, split, extract, rotate, watermark and number PDF pages offline. One Rust core, JavaScript, Python and Rust bindings, plus a CLI.',
					},
				},
				{ tag: 'meta', attrs: { name: 'twitter:image', content: 'https://dusrelpdf.pages.dev/og.svg' } },
				{
					tag: 'script',
					attrs: { type: 'application/ld+json' },
					content: JSON.stringify(jsonLd),
				},
			],
			editLink: {
				baseUrl: 'https://github.com/dusmamud/dusrelpdf/edit/main/website/',
			},
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Overview', slug: 'overview' },
						{ label: 'Installation', slug: 'guides/installation' },
						{ label: 'Quickstart', slug: 'guides/quickstart' },
						{ label: 'Options', slug: 'guides/options' },
						{ label: 'Error codes', slug: 'guides/error-codes' },
					],
				},
				{
					label: 'Operations',
					items: [
						{ label: 'Merge', slug: 'reference/merge' },
						{ label: 'Split', slug: 'reference/split' },
						{ label: 'Extract', slug: 'reference/extract' },
						{ label: 'Remove', slug: 'reference/remove' },
						{ label: 'Reorder', slug: 'reference/reorder' },
						{ label: 'Rotate', slug: 'reference/rotate' },
						{ label: 'Images to PDF', slug: 'reference/images-to-pdf' },
						{ label: 'Watermark', slug: 'reference/watermark' },
						{ label: 'Page numbers', slug: 'reference/page-numbers' },
						{ label: 'Signature', slug: 'reference/signature' },
						{ label: 'Info & bulk', slug: 'reference/info-and-bulk' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'CLI', slug: 'reference/cli' },
						{ label: 'Rust API', slug: 'reference/rust-api' },
						{ label: 'JavaScript API', slug: 'reference/js-api' },
						{ label: 'Python API', slug: 'reference/python-api' },
						{ label: 'Limitations', slug: 'reference/limitations' },
						{ label: 'Comparison', slug: 'comparison' },
						{ label: 'Changelog', slug: 'changelog' },
					],
				},
			],
		}),
		sitemap(),
	],
});
