import { useEffect, useRef, useState } from 'preact/hooks';

export default function Hero() {
  const [isVideoPlaying, setIsVideoPlaying] = useState(false);
  const videoRef = useRef(null);
  const heroImgRef = useRef(null);
  const doNotTouchRef = useRef(null);
  const eyesRef = useRef([]);

  useEffect(() => {
    function handleMouseMove(e) {
      requestAnimationFrame(() => {
        eyesRef.current.forEach((eye) => {
          if (!eye) return;
          const container = eye.parentElement;
          const containerRect = container.getBoundingClientRect();
          const eyeRect = eye.getBoundingClientRect();
          const containerCenterX = containerRect.left + containerRect.width / 2;
          const containerCenterY = containerRect.top + containerRect.height / 2;
          const deltaX = e.clientX - containerCenterX;
          const deltaY = e.clientY - containerCenterY;
          const angle = Math.atan2(deltaY, deltaX);
          const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
          const maxMoveX = (containerRect.width - eyeRect.width) / 2;
          const maxMoveY = (containerRect.height - eyeRect.height) / 2;
          const movementScale = Math.min(distance / 200, 1);
          const moveX = Math.cos(angle) * maxMoveX * movementScale;
          const moveY = Math.sin(angle) * maxMoveY * movementScale;
          eye.style.transform = `translate(${moveX}px, ${moveY}px)`;
        });
      });
    }

    function handleMouseLeave() {
      eyesRef.current.forEach((eye) => {
        if (eye) eye.style.transform = 'translate(0, 0)';
      });
    }

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseleave', handleMouseLeave);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, []);

  function handleDoNotTouchClick() {
    if (isVideoPlaying) return;
    const video = videoRef.current;
    const heroImg = heroImgRef.current;
    if (!video || !heroImg) return;

    setIsVideoPlaying(true);
    video.load();
    video.style.display = 'block';
    video.play();
    heroImg.style.opacity = '0';

    function onEnded() {
      video.removeEventListener('ended', onEnded);
      video.style.display = 'none';
      heroImg.style.opacity = '1';
      video.currentTime = 0;
      setIsVideoPlaying(false);
    }

    video.addEventListener('ended', onEnded);
  }

  return (
    <section class="flex items-center" style={{ paddingBottom: '32px', background: '#ffffff' }}>
      <div class="container max-w-6xl mx-auto px-6 text-center">
        <div class="relative mx-auto" style={{ maxWidth: '1280px', width: '100%' }}>
          <div class="mb-8 relative inline-block">
            <img
              ref={heroImgRef}
              src="/images/hero.png"
              alt="Solobase Dashboard"
              class="mx-auto rounded-lg"
              style={{ maxWidth: '400px', width: '100%', height: 'auto', position: 'relative' }}
            />
            <video
              ref={videoRef}
              class="absolute top-0 left-0 rounded-lg"
              style={{ maxWidth: '400px', width: '100%', height: '100%', objectFit: 'cover', display: 'none', zIndex: 25 }}
            >
              <source src="/videos/video_1.mp4" type="video/mp4" />
            </video>
            {/* Eye containers */}
            <div class="absolute flex justify-center items-center" style={{ bottom: '28%', left: '52%', transform: 'translateX(-50%)', gap: '12px', zIndex: 20 }}>
              {[0, 1].map((i) => (
                <div key={i} style={{ width: '24px', height: '24px', position: 'relative', display: 'flex', alignItems: 'center', justifyContent: 'center', overflow: 'hidden' }}>
                  <img
                    ref={(el) => (eyesRef.current[i] = el)}
                    src="/images/eye.png"
                    alt="Eye"
                    style={{ width: '12px', height: '12px', maxWidth: '12px', objectFit: 'contain', position: 'relative' }}
                  />
                </div>
              ))}
            </div>
          </div>
          <img
            ref={doNotTouchRef}
            src="/images/do_not_touch.png"
            alt="Do not touch"
            onClick={handleDoNotTouchClick}
            class="absolute"
            style={{
              bottom: '40px',
              right: 0,
              width: '80px',
              height: 'auto',
              zIndex: 15,
              cursor: 'pointer',
              transformOrigin: 'center center',
              transition: 'transform 0.2s ease',
              opacity: isVideoPlaying ? 0.5 : 1,
              pointerEvents: isVideoPlaying ? 'none' : 'auto',
            }}
            onMouseOver={(e) => { if (!isVideoPlaying) e.currentTarget.style.animation = 'wobble 0.8s ease-in-out infinite'; }}
            onMouseOut={(e) => { e.currentTarget.style.animation = 'none'; }}
          />
        </div>
        <h1 class="text-5xl md:text-6xl font-bold mb-6" style={{ marginTop: 0, color: '#1f2937', letterSpacing: '-0.02em' }}>
          Solobase
        </h1>
        <p class="text-xl md:text-2xl mb-12 max-w-2xl mx-auto" style={{ color: '#6b7280' }}>
          A fast backend without the overcomplicated setup or dependencies. Just a single binary.
        </p>
      </div>
    </section>
  );
}
