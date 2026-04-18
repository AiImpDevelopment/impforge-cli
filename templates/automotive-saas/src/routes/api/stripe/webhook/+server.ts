// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
import Stripe from "stripe";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY!);
const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET!;

// Timing-safe signature check — must use raw request body, not
// JSON-parsed content, or the signature never matches.
export const POST = async ({ request }: { request: Request }) => {
  const signature = request.headers.get("stripe-signature") ?? "";
  const rawBody = await request.text();
  let event: Stripe.Event;
  try {
    event = stripe.webhooks.constructEvent(rawBody, signature, webhookSecret);
  } catch {
    return new Response("invalid signature", { status: 400 });
  }

  switch (event.type) {
    case "customer.subscription.updated":
    case "customer.subscription.deleted":
      // TODO: update local subscriptions table under withTenant()
      break;
  }
  return new Response("ok", { status: 200 });
};
