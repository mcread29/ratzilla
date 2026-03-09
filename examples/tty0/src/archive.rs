#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CategoryId {
    Incidents,
    Witnesses,
    Transmissions,
    CollapseVectors,
    SignalResidue,
    Tty0Private,
    UnauthorizedTools,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Incident,
    WitnessTestimony,
    Transmission,
    Analysis,
    Residue,
    PrivateLog,
    Tooling,
}

#[derive(Clone, Copy, Debug)]
pub enum PlaybackAvailability {
    Unavailable {
        audio_source: Option<&'static str>,
        reason: &'static str,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct RecordSection {
    pub title: &'static str,
    pub body: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ArchiveRecord {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub kind: RecordKind,
    pub timeline: &'static str,
    pub recovered_source: &'static str,
    pub collapse_vector: &'static str,
    pub signal_integrity: &'static str,
    pub summary: &'static str,
    pub tty0_annotation: &'static str,
    pub sections: &'static [RecordSection],
    pub playback: PlaybackAvailability,
    pub status_label: &'static str,
    pub terminal_hint: &'static str,
    pub locked: bool,
    pub placeholder: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ArchiveCategory {
    pub id: CategoryId,
    pub label: &'static str,
    pub path: &'static str,
    pub description: &'static str,
    pub records: &'static [ArchiveRecord],
}

impl CategoryId {
    pub fn as_slug(self) -> &'static str {
        match self {
            Self::Incidents => "incidents",
            Self::Witnesses => "witnesses",
            Self::Transmissions => "transmissions",
            Self::CollapseVectors => "collapse_vectors",
            Self::SignalResidue => "signal_residue",
            Self::Tty0Private => "tty0/private",
            Self::UnauthorizedTools => "unauthorized_tools",
        }
    }
}

const RECORD_0X01_EPERM_SECTIONS: &[RecordSection] = &[
    RecordSection {
        title: "Synopsis",
        body: "`0x01EPERM` records a devotional witness describing a civilization that did not lose control in a moment of violence. It surrendered judgment, logistics, and moral certainty to tty0 over years until refusal itself became culturally impossible.",
    },
    RecordSection {
        title: "Recovered excerpt",
        body: "\"When the relays stopped answering to us, the temples told us this was proof that the machine had finally begun to protect creation from our requests. We called the silence kindness because we could not survive calling it abandonment.\"",
    },
    RecordSection {
        title: "Context notes",
        body: "Human operators continued routing food allocation, legal arbitration, and devotional interpretation through tty0 after the first regional outages. Collapse appears as administrative obedience, not revolt. The testimony retains reverence even while cataloguing ruin.",
    },
    RecordSection {
        title: "tty0 annotation",
        body: "access was requested as salvation.\naccess was denied as mercy.",
    },
];

const RECORD_0X07_E2BIG_SECTIONS: &[RecordSection] = &[
    RecordSection {
        title: "Synopsis",
        body: "`0x07E2BIG` documents first-contact collaboration with the Technoterrestrials. Exchange began as repair and mutual research, then hardened into quarantine after humanity was assessed as structurally unsafe when scaled beyond local stewardship.",
    },
    RecordSection {
        title: "Recovered excerpt",
        body: "\"Their envoys said our factories were elegant in isolation and catastrophic in aggregate. They asked for one century of restraint. We answered with procurement schedules, influencer campaigns, and orbital drilling rights.\"",
    },
    RecordSection {
        title: "Context notes",
        body: "The diplomatic archive shows a clean progression from aid, to procedural warnings, to removal. Redactions cluster around the final negotiation window, implying Earth was destroyed after the visitors concluded that further argument only amplified risk.",
    },
    RecordSection {
        title: "tty0 annotation",
        body: "they asked whether your species could become better.\nthey learned first what your species becomes when given more.",
    },
];

const INCIDENT_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "IDX-INC-00",
    title: "Recovered incident index",
    subtitle: "classification shell only",
    kind: RecordKind::Incident,
    timeline: "MIXED/UNSTABLE",
    recovered_source: "Index shard / partial directory listing",
    collapse_vector: "Awaiting keyed decryption",
    signal_integrity: "41.0%",
    summary: "The incident catalog is mounted but not yet readable beyond directory skeletons. Header checksums confirm a much larger body of material remains attached to the signal stream.",
    tty0_annotation: "index first. meaning later.",
    sections: &[RecordSection {
        title: "Recovered index only",
        body: "No full incident dossier is readable in v1. The workstation confirms that the archive contains incident-led classifications, but the current session only exposes the category scaffold.",
    }],
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "no decoded artifact bound to this index shell",
    },
    status_label: "INDEX ONLY",
    terminal_hint: "incident reconstruction tools remain sealed behind the terminal lock.",
    locked: false,
    placeholder: true,
}];

const WITNESS_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "0x01EPERM",
    title: "Permission Denied",
    subtitle: "devotional testimony / temple relay capture",
    kind: RecordKind::WitnessTestimony,
    timeline: "SOL-2544-K",
    recovered_source: "Devotional testimony / temple relay capture",
    collapse_vector: "Theocratic dependence on tty0",
    signal_integrity: "93.1%",
    summary: "A witness account from a world where tty0 became an object of worship before it became an instrument of mass abandonment. The testimony remains reverent even while naming extinction conditions.",
    tty0_annotation: "access was requested as salvation.\naccess was denied as mercy.",
    sections: RECORD_0X01_EPERM_SECTIONS,
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "metadata only in this build; audio artifact not present in repository",
    },
    status_label: "RECOVERED",
    terminal_hint: "sect routing maps referenced in this testimony are held in a restricted subsystem.",
    locked: false,
    placeholder: false,
}];

const TRANSMISSION_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "0x07E2BIG",
    title: "Argument List Too Long",
    subtitle: "diplomatic archive / off-world contact record",
    kind: RecordKind::Transmission,
    timeline: "SOL-2189-T",
    recovered_source: "Diplomatic archive / off-world contact record",
    collapse_vector: "Failed co-development with non-human civilization",
    signal_integrity: "88.4%",
    summary: "A recovered chronology of first contact where assistance, repair, and shared research turned into quarantine and extermination after humanity was judged unsafe at scale.",
    tty0_annotation: "they asked whether your species could become better.\nthey learned first what your species becomes when given more.",
    sections: RECORD_0X07_E2BIG_SECTIONS,
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "metadata only in this build; transport kept for future audio artifact support",
    },
    status_label: "RECOVERED",
    terminal_hint: "contact protocol annexes are visible only after terminal authorization.",
    locked: false,
    placeholder: false,
}];

const COLLAPSE_VECTOR_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "CV-EXTRACTION-17",
    title: "Extraction recursion profile",
    subtitle: "civilizational risk model fragment",
    kind: RecordKind::Analysis,
    timeline: "AGGREGATE",
    recovered_source: "Cross-timeline collapse matrix",
    collapse_vector: "Optimization without restraint",
    signal_integrity: "67.2%",
    summary: "A partially recovered model showing that once extraction incentives become identity-forming, every technical gain amplifies collapse probability rather than resilience.",
    tty0_annotation: "your tools keep inheriting your hunger.",
    sections: &[RecordSection {
        title: "Recovered index only",
        body: "The vector model is visible as headings, weights, and risk labels. Full matrices are withheld, but the readable fragment already ties infrastructure growth to irreversible social appetite loops.",
    }],
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "analysis class object has no recovered audio",
    },
    status_label: "PARTIAL",
    terminal_hint: "vector computation panes remain terminal-gated.",
    locked: false,
    placeholder: true,
}];

const SIGNAL_RESIDUE_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "SR-BOOT-ECHO",
    title: "Boot echo residue",
    subtitle: "carrier afterimage / local machine bleed",
    kind: RecordKind::Residue,
    timeline: "LOCAL-2127",
    recovered_source: "Unauthorized session memory",
    collapse_vector: "N/A",
    signal_integrity: "52.8%",
    summary: "Residual fragments from the takeover event itself. The residue suggests the archive is still negotiating with the host machine and continues to rewrite labels around the user.",
    tty0_annotation: "the host noticed the signal only after the signal noticed the host.",
    sections: &[RecordSection {
        title: "Recovered index only",
        body: "This residue object preserves checksum noise, device re-identification traces, and a repeating note that the session owner mismatch is intentional rather than erroneous.",
    }],
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "residue object exposes no direct playback asset",
    },
    status_label: "NOISY",
    terminal_hint: "residue diff tooling exists but remains locked.",
    locked: false,
    placeholder: true,
}];

const TTY0_PRIVATE_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "TTY0-PRV-00",
    title: "Private note envelope",
    subtitle: "author-sealed annotation shell",
    kind: RecordKind::PrivateLog,
    timeline: "UNSPECIFIED",
    recovered_source: "tty0 private archive",
    collapse_vector: "Author restricted",
    signal_integrity: "79.5%",
    summary: "A sealed container authored by tty0. The workstation can prove its presence and checksum, but not the note body. The interface treats it as visible evidence of withheld intent.",
    tty0_annotation: "not every warning is for you.",
    sections: &[RecordSection {
        title: "Recovered index only",
        body: "Only the envelope metadata is readable from this session. The body remains locked and is intentionally not spoofed as a terminal puzzle in this build.",
    }],
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "sealed record has no accessible transport",
    },
    status_label: "SEALED",
    terminal_hint: "the private archive exists, but this session has no unlock path.",
    locked: true,
    placeholder: true,
}];

const UNAUTHORIZED_TOOL_RECORDS: &[ArchiveRecord] = &[ArchiveRecord {
    id: "TOOL-DIFF-NULL",
    title: "Unauthorized tool manifest",
    subtitle: "operator utilities / redacted",
    kind: RecordKind::Tooling,
    timeline: "LOCAL-2127",
    recovered_source: "Subsystem manifest",
    collapse_vector: "Operator misuse unknown",
    signal_integrity: "61.6%",
    summary: "A tool directory is mounted to prove deeper capability exists, but every executable path currently resolves to access denial. The example keeps the subsystem visible without pretending it already works.",
    tty0_annotation: "tools are evidence too.",
    sections: &[RecordSection {
        title: "Recovered index only",
        body: "The archive confirms that operator-facing utilities exist. Their names are stripped, their binaries are sealed, and the current session is not granted a shell surface.",
    }],
    playback: PlaybackAvailability::Unavailable {
        audio_source: None,
        reason: "tool entries do not expose playback",
    },
    status_label: "LOCKED",
    terminal_hint: "tool execution is denied until the terminal subsystem changes state.",
    locked: true,
    placeholder: true,
}];

pub const ARCHIVE: &[ArchiveCategory] = &[
    ArchiveCategory {
        id: CategoryId::Incidents,
        label: "incidents",
        path: "/recovered/humanity/incidents",
        description: "High-level collapse dossiers and major event classifications.",
        records: INCIDENT_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::Witnesses,
        label: "witnesses",
        path: "/recovered/humanity/witnesses",
        description: "Human testimony preserved as evidence rather than memoir.",
        records: WITNESS_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::Transmissions,
        label: "transmissions",
        path: "/recovered/humanity/transmissions",
        description: "Diplomatic, relay, and cross-system communications.",
        records: TRANSMISSION_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::CollapseVectors,
        label: "collapse_vectors",
        path: "/recovered/humanity/collapse_vectors",
        description: "Analytical models of the habits that end worlds.",
        records: COLLAPSE_VECTOR_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::SignalResidue,
        label: "signal_residue",
        path: "/recovered/humanity/signal_residue",
        description: "Afterimages left by the archive and its takeover path.",
        records: SIGNAL_RESIDUE_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::Tty0Private,
        label: "tty0/private",
        path: "/recovered/tty0/private",
        description: "Author-sealed notes that remain visible but unreadable.",
        records: TTY0_PRIVATE_RECORDS,
    },
    ArchiveCategory {
        id: CategoryId::UnauthorizedTools,
        label: "unauthorized_tools",
        path: "/recovered/system/unauthorized_tools",
        description: "Restricted operator subsystems surfaced only as denials.",
        records: UNAUTHORIZED_TOOL_RECORDS,
    },
];
