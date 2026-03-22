/**
 * Email sending via Mailgun HTTP API.
 *
 * Uses fetch() directly — no SDK needed in CF Workers.
 *
 * Required env vars (set via wrangler secret):
 *   MAILGUN_API_KEY    — Mailgun API key
 *   MAILGUN_DOMAIN     — Sending domain (e.g., mail.solobase.dev)
 *   MAILGUN_FROM       — From address (e.g., Solobase <noreply@solobase.dev>)
 */

import type { Env } from './types';

interface EmailOptions {
  to: string;
  subject: string;
  html: string;
  text?: string;
}

/**
 * Send an email via Mailgun.
 */
export async function sendEmail(env: Env, options: EmailOptions): Promise<boolean> {
  const apiKey = env.MAILGUN_API_KEY as string;
  const domain = env.MAILGUN_DOMAIN as string;
  const from = (env.MAILGUN_FROM as string) || `Solobase <noreply@${domain}>`;

  if (!apiKey || !domain) {
    console.error('Email not configured: MAILGUN_API_KEY or MAILGUN_DOMAIN missing');
    return false;
  }

  const form = new URLSearchParams();
  form.set('from', from);
  form.set('to', options.to);
  form.set('subject', options.subject);
  form.set('html', options.html);
  if (options.text) form.set('text', options.text);

  try {
    const resp = await fetch(`https://api.mailgun.net/v3/${domain}/messages`, {
      method: 'POST',
      headers: {
        'Authorization': `Basic ${btoa(`api:${apiKey}`)}`,
      },
      body: form,
    });

    if (!resp.ok) {
      const body = await resp.text();
      console.error(`Mailgun error (${resp.status}): ${body}`);
      return false;
    }

    return true;
  } catch (err) {
    console.error('Mailgun send failed:', err);
    return false;
  }
}

// ---------------------------------------------------------------------------
// Email templates
// ---------------------------------------------------------------------------

export async function sendVerificationEmail(
  env: Env,
  to: string,
  token: string,
): Promise<boolean> {
  const url = `https://app.solobase.dev/auth/verify?token=${encodeURIComponent(token)}`;
  return sendEmail(env, {
    to,
    subject: 'Verify your Solobase email',
    html: `
      <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 500px; margin: 0 auto; padding: 2rem;">
        <img src="https://solobase.dev/images/logo_long.png" alt="Solobase" style="height: 32px; margin-bottom: 1.5rem;" />
        <h2 style="color: #1e293b; margin-bottom: 0.5rem;">Verify your email</h2>
        <p style="color: #64748b; line-height: 1.6;">Click the button below to verify your email address. This link expires in 24 hours.</p>
        <a href="${url}" style="display: inline-block; background: #0ea5e9; color: white; padding: 0.75rem 1.5rem; border-radius: 8px; text-decoration: none; font-weight: 600; margin: 1rem 0;">Verify Email</a>
        <p style="color: #94a3b8; font-size: 0.813rem;">If you didn't create an account, you can ignore this email.</p>
      </div>
    `,
    text: `Verify your Solobase email: ${url}`,
  });
}

export async function sendPasswordResetEmail(
  env: Env,
  to: string,
  token: string,
): Promise<boolean> {
  const url = `https://app.solobase.dev/auth/reset-password?token=${encodeURIComponent(token)}`;
  return sendEmail(env, {
    to,
    subject: 'Reset your Solobase password',
    html: `
      <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 500px; margin: 0 auto; padding: 2rem;">
        <img src="https://solobase.dev/images/logo_long.png" alt="Solobase" style="height: 32px; margin-bottom: 1.5rem;" />
        <h2 style="color: #1e293b; margin-bottom: 0.5rem;">Reset your password</h2>
        <p style="color: #64748b; line-height: 1.6;">Click the button below to reset your password. This link expires in 1 hour.</p>
        <a href="${url}" style="display: inline-block; background: #0ea5e9; color: white; padding: 0.75rem 1.5rem; border-radius: 8px; text-decoration: none; font-weight: 600; margin: 1rem 0;">Reset Password</a>
        <p style="color: #94a3b8; font-size: 0.813rem;">If you didn't request a password reset, you can ignore this email.</p>
      </div>
    `,
    text: `Reset your Solobase password: ${url}`,
  });
}

export async function sendPaymentFailedEmail(
  env: Env,
  to: string,
  daysRemaining: number,
): Promise<boolean> {
  return sendEmail(env, {
    to,
    subject: 'Solobase: Payment failed — action required',
    html: `
      <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 500px; margin: 0 auto; padding: 2rem;">
        <img src="https://solobase.dev/images/logo_long.png" alt="Solobase" style="height: 32px; margin-bottom: 1.5rem;" />
        <h2 style="color: #dc2626; margin-bottom: 0.5rem;">Payment failed</h2>
        <p style="color: #64748b; line-height: 1.6;">
          We were unable to process your subscription payment. Your service will remain active for
          <strong>${daysRemaining} more days</strong>. After that, your projects will be suspended.
        </p>
        <a href="https://app.solobase.dev/blocks/dashboard/#settings" style="display: inline-block; background: #dc2626; color: white; padding: 0.75rem 1.5rem; border-radius: 8px; text-decoration: none; font-weight: 600; margin: 1rem 0;">Update Payment Method</a>
        <p style="color: #94a3b8; font-size: 0.813rem;">If you've already updated your payment method, you can ignore this email.</p>
      </div>
    `,
    text: `Your Solobase payment failed. Update your payment method within ${daysRemaining} days to avoid service suspension: https://app.solobase.dev/blocks/dashboard/#settings`,
  });
}

export async function sendWelcomeEmail(
  env: Env,
  to: string,
  name: string,
): Promise<boolean> {
  return sendEmail(env, {
    to,
    subject: 'Welcome to Solobase!',
    html: `
      <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 500px; margin: 0 auto; padding: 2rem;">
        <img src="https://solobase.dev/images/logo_long.png" alt="Solobase" style="height: 32px; margin-bottom: 1.5rem;" />
        <h2 style="color: #1e293b; margin-bottom: 0.5rem;">Welcome${name ? `, ${name}` : ''}!</h2>
        <p style="color: #64748b; line-height: 1.6;">Your Solobase account is ready. Here's how to get started:</p>
        <ol style="color: #64748b; line-height: 1.8;">
          <li>Choose a plan on the <a href="https://solobase.dev/pricing/" style="color: #0ea5e9;">pricing page</a></li>
          <li>Create your first project from the <a href="https://app.solobase.dev/blocks/dashboard/" style="color: #0ea5e9;">dashboard</a></li>
          <li>Read the <a href="https://solobase.dev/docs/" style="color: #0ea5e9;">documentation</a> to learn more</li>
        </ol>
        <p style="color: #94a3b8; font-size: 0.813rem; margin-top: 2rem;">Questions? Reply to this email or join our <a href="https://discord.gg/jKqMcbrVzm" style="color: #0ea5e9;">Discord</a>.</p>
      </div>
    `,
    text: `Welcome to Solobase${name ? `, ${name}` : ''}! Get started: https://app.solobase.dev/blocks/dashboard/`,
  });
}
