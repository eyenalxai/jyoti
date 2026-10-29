import { MiioError } from "@/lib/miio/error"

const STREAM_TIMEOUT_MS = 600

const timeoutError = (address: string, kind: "hello" | "stream"): MiioError =>
  new MiioError({
    kind: "timeout",
    address,
    message:
      kind === "hello"
        ? `No reply from ${address} to the miIO hello. Check its power and network.`
        : `No reply from ${address} within ${STREAM_TIMEOUT_MS} ms.`,
  })

export { STREAM_TIMEOUT_MS, timeoutError }
