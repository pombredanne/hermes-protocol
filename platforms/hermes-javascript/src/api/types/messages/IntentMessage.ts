import { slotType, slotType_legacy, grain } from '../enums'

export type CustomSlotValue<T extends slotType.custom> = {
    kind: T,
    value: string
}
export type NumberSlotValue<T extends slotType.number> = {
    kind: T,
    value: number
}
export type OrdinalSlotValue<T extends slotType.ordinal> = {
    kind: T,
    value: number
}
export type InstantTimeSlotValue<T extends slotType.instantTime> = {
    kind: T,
    value: string,
    grain: grain,
    precisition: 'Exact' | 'Approximate'
}
export type TimeIntervalSlotValue<T extends slotType.timeInterval> = {
    kind: T,
    from: string,
    to: string
}
export type AmountOfMoneySlotValue<T extends slotType.amountOfMoney> = {
    kind: T,
    value: number,
    precisition: 'Exact' | 'Approximate',
    unit: string
}
export type TemperatureSlotValue<T extends slotType.temperature> = {
    kind: T,
    value: number,
    unit: 'celsius' | 'fahrenheit'
}
export type DurationSlotValue<T extends slotType.duration> = {
    kind: T,
    years: number,
    quarters: number,
    months: number,
    weeks: number,
    days: number,
    hours: number,
    minutes: number,
    seconds: number,
    precision: 'Exact' | 'Approximate'
}
export type MusicAlbumSlotValue<T extends slotType.musicAlbum> = {
    kind: T,
    value: string
}
export type MusicArtistSlotValue<T extends slotType.musicArtist> = {
    kind: T,
    value: string
}
export type MusicTrackSlotValue<T extends slotType.musicTrack> = {
    kind: T,
    value: string
}

export type NluSlot<T extends slotType = slotType> = {
    confidenceScore: number,
    rawValue: string,
    range: {
        start: number,
        end: number
    },
    entity: string,
    slotName: string,
    value:
        T extends slotType.custom ? CustomSlotValue<T> :
        T extends slotType.number ? NumberSlotValue<T> :
        T extends slotType.ordinal ? OrdinalSlotValue<T> :
        T extends slotType.instantTime ? InstantTimeSlotValue<T> :
        T extends slotType.timeInterval ? TimeIntervalSlotValue<T> :
        T extends slotType.amountOfMoney ? AmountOfMoneySlotValue<T> :
        T extends slotType.temperature ? TemperatureSlotValue<T> :
        T extends slotType.duration ? DurationSlotValue<T> :
        T extends slotType.musicAlbum ? MusicAlbumSlotValue<T> :
        T extends slotType.musicArtist ? MusicArtistSlotValue<T> :
        T extends slotType.musicTrack ? MusicTrackSlotValue<T> :
        never
}

export type IntentMessage = {
    sessionId: string,
    siteId: string,
    input: string,
    customData?: string,
    intent: {
        intentName: string,
        confidenceScore: number
    },
    asrTokens: [
        {
            value: string,
            confidence: number,
            rangeStart: number,
            rangeEnd: number,
            time: {
                start: number,
                end: number
            }
        }[]?
    ],
    slots: NluSlot[]
}

export type IntentMessageLegacy = {
    session_id: string,
    custom_data?: string,
    site_id: string,
    input: string,
    intent: {
        intent_name: string,
        confidence_score: number
    },
    asr_tokens: [
        {
            value: string,
            confidence: number,
            range_start: number,
            range_end: number,
            time: {
                start: number,
                end: number
            }
        }[]?
    ],
    slots: {
        confidence_score: number,
        raw_value: string,
        range_start: number,
        range_end: number
        entity: string,
        slot_name: string,
        value: {
            value_type: slotType_legacy,
            // Wildcard
            value: any
        }
    }[]
}