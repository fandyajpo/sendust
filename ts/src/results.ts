import { SendustInternalError } from "./errors.ts";
import type { NativeSendResult } from "./loader.ts";
import type { DeliveryStatus, RecipientStatus, SendResult } from "./types.ts";

const DELIVERY_STATUSES: readonly DeliveryStatus[] = ["accepted", "rejected", "unknown"];
const RECIPIENT_STATUSES: readonly RecipientStatus[] = ["accepted", "rejected", "unknown"];

function internal(message: string): SendustInternalError {
  return new SendustInternalError({ code: "INTERNAL", category: "internal", message });
}

function toDeliveryStatus(value: string): DeliveryStatus {
  if ((DELIVERY_STATUSES as readonly string[]).includes(value)) {
    return value as DeliveryStatus;
  }
  throw internal(`unexpected native delivery status: ${value}`);
}

function toRecipientStatus(value: string): RecipientStatus {
  if ((RECIPIENT_STATUSES as readonly string[]).includes(value)) {
    return value as RecipientStatus;
  }
  throw internal(`unexpected native recipient status: ${value}`);
}

/**
 * Maps the native payload to the public result without reinterpreting SMTP
 * outcomes: `accepted`, `rejected`, and `unknown` pass through verbatim, each
 * recipient keeps the status the relay gave it, and no delivery claim is added.
 */
export function toSendResult(result: NativeSendResult): SendResult {
  const mapped: SendResult = {
    status: toDeliveryStatus(result.status),
    recipients: result.recipients.map((recipient) => ({
      address: recipient.address,
      status: toRecipientStatus(recipient.status),
    })),
  };
  return result.response ? { ...mapped, response: result.response } : mapped;
}
