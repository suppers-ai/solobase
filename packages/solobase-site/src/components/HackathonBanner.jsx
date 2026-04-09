import { siteConfig } from '../data/navigation';

export default function HackathonBanner() {
  return (
    <div class="container max-w-6xl mx-auto px-6" style={{ paddingTop: '2rem', paddingBottom: '1rem' }}>
      <div
        style={{
          background: '#8c40ff',
          color: 'white',
          padding: '1rem 1.5rem',
          textAlign: 'center',
          borderRadius: '0.75rem',
          boxShadow: '0 2px 8px rgba(140, 64, 255, 0.2)',
        }}
      >
        <p style={{ margin: 0, fontSize: '0.95rem', lineHeight: 1.5 }}>
          This project was built for a hackathon by{' '}
          <a href="https://kiro.dev/" target="_blank" rel="noopener noreferrer" style={{ color: 'white', textDecoration: 'underline', fontWeight: 600 }}>
            Kiro
          </a>{' '}
          to test out the IDE, it is by no means complete or production ready.
          <br />
          Please{' '}
          <a href={siteConfig.githubUrl} target="_blank" rel="noopener noreferrer" style={{ color: 'white', textDecoration: 'underline', fontWeight: 600 }}>
            star the project
          </a>{' '}
          and{' '}
          <a href={siteConfig.discordUrl} target="_blank" rel="noopener noreferrer" style={{ color: 'white', textDecoration: 'underline', fontWeight: 600 }}>
            join Discord
          </a>{' '}
          if you think this will be useful and want me to continue developing it.
        </p>
      </div>
    </div>
  );
}
