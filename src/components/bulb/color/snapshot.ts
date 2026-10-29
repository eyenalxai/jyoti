import type { BulbId, BulbState } from "@/lib/bulbs/schema"

type ColorSnapshot = ReadonlyMap<BulbId, BulbState | null>

export type { ColorSnapshot }
