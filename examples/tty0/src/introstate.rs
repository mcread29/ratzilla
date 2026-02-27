use crate::state::{StateActions, StateMachineError};
use ratzilla::event::KeyCode;
use ratzilla::ratatui::layout::{Constraint, Layout};
use ratzilla::ratatui::style::{Color, Modifier, Style};
use ratzilla::ratatui::text::{self, Span};
use ratzilla::ratatui::widgets::{Block, Gauge};
use ratzilla::ratatui::Frame;
use tachyonfx::Duration;

use ratzilla::ratatui::layout::Rect;
use ratzilla::ratatui::widgets::{List, ListItem, ListState};

use crate::logo_text::render_logo_text;

const BOOT: &str = r#"
PMAP: PCID enabled
Hacknet Kernel Version 1.0.0: Tue Oct 11 20:56:35 PDT 2011; root:xnu-1699.22.73~1/RELEASE_X86_64
vm_page_bootstrap: 987323 free pages and 53061 wired pages
kext submap [0xffffff7f8072e000 - 0xffffff8000000000], kernel text [0xffffff8000200000 - 0xffffff800072e000]
zone leak detection enabled
standard timeslicing quantum is 10000 us
mig_table_max_displ = 72
TSC Deadline Timer supported and enabled
HacknetACPICPU: ProcessorId=1 LocalApicId=0 Enabled
HacknetACPICPU: ProcessorId=2 LocalApicId=2 Enabled
HacknetACPICPU: ProcessorId=3 LocalApicId=1 Enabled
HacknetACPICPU: ProcessorId=4 LocalApicId=3 Enabled
HacknetACPICPU: ProcessorId=5 LocalApicId=255 Disabled
HacknetACPICPU: ProcessorId=6 LocalApicId=255 Disabled
HacknetACPICPU: ProcessorId=7 LocalApicId=255 Disabled
HacknetACPICPU: ProcessorId=8 LocalApicId=255 Disabled
calling mpo_policy_init for TMSafetyNet
Security policy loaded: Safety net for Rollback (TMSafetyNet)
calling mpo_policy_init for Sandbox
Security policy loaded: Seatbelt sandbox policy (Sandbox)
calling mpo_policy_init for Quarantine
Security policy loaded: Quarantine policy (Quarantine)
Copyright (c) 1982, 1986, 1989, 1991, 1993, 2015
The Regents of the University of Adelaide. All rights reserved.
 
HN_ Framework successfully initialized
using 16384 buffer headers and 10240 cluster IO buffer headers
IOAPIC: Version 0x20 Vectors 64:87
ACPI: System State [S0 S3 S4 S5] (S3)
PFM64 0xf10000000, 0xf0000000
[ PCI configuration begin ]
HacknetIntelCPUPowerManagement: Turbo Ratios 0046
HacknetIntelCPUPowerManagement: (built 13:08:12 Jun 18 2011) initialization complete
console relocated to 0xf10000000
PCI configuration changed (bridge=16 device=4 cardbus=0)
[ PCI configuration end, bridges 12 devices 16 ]
mbinit: done [64 MB total pool size, (42/21) split]
Pthread support ABORTS when sync kernel primitives misused
com.Hacknet.HacknetFSCompressionTypeZlib kmod start
com.Hacknet.HacknetTrololoBootScreen kmod start
com.Hacknet.HacknetFSCompressionTypeZlib load succeeded
com.Hacknet.HacknetFSCompressionTypeDataless load succeeded
 
HacknetIntelCPUPowerManagementClient: ready
BTCOEXIST off 
wl0: Broadcom BCM4331 802.11 Wireless Controller
5.100.98.75
 
FireWire (OHCI) Lucent ID 5901 built-in now active, GUID c82a14fffee4a086; max speed s800.
rooting via boot-uuid from /chosen: F5670083-AC74-33D3-8361-AC1977EE4AA2
Waiting on <dict ID="0"><key>IOProviderClass</key><string ID="1">
IOResources</string><key>IOResourceMatch</key><string ID="2">boot-uuid-media</string></dict>
Got boot device = IOService:/HacknetACPIPlatformExpert/PCI0@0/HacknetACPIPCI/SATA@1F,2/
HacknetIntelPchSeriesAHCI/PRT0@0/IOAHCIDevice@0/HacknetAHCIDiskDriver/SarahI@sTheBestDriverIOAHCIBlockStorageDevice/IOBlockStorageDriver/
Hacknet SSD TS128C Media/IOGUIDPartitionScheme/Customer@2
BSD root: disk0s2, major 14, minor 2
Kernel is LP64
IOThunderboltSwitch::i2cWriteDWord - status = 0xe00002ed
IOThunderboltSwitch::i2cWriteDWord - status = 0x00000000
IOThunderboltSwitch::i2cWriteDWord - status = 0xe00002ed
IOThunderboltSwitch::i2cWriteDWord - status = 0xe00002ed
HacknetUSBMultitouchDriver::checkStatus - received Status Packet, Payload 2: device was reinitialized
MottIsAScrub::checkstatus - true, Mott::Scrub
[IOBluetoothHCIController::setConfigState] calling registerService
AirPort_Brcm4331: Ethernet address e4:ce:8f:46:18:d2
IO80211Controller::dataLinkLayerAttachComplete():  adding HacknetEFINVRAM notification
IO80211Interface::efiNVRAMPublished():  
Created virtif 0xffffff800c32ee00 p2p0
BCM5701Enet: Ethernet address c8:2a:14:57:a4:7a
Previous Shutdown Cause: 3
NTFS driver 3.8 [Flags: R/W].
NTFS volume name BOOTCAMP, version 3.1.
DSMOS has arrived
en1: 802.11d country code set to 'US'.
en1: Supported channels 1 2 3 4 5 6 7 8 9 10 11 36 40 44 48 52 56 60 64 100 104 108 112 116 120 124 128 132 136 140 149 153 157 161 165
m_thebest 
MacAuthEvent en1   Auth result for: 00:60:64:1e:e9:e4  MAC AUTH succeeded
MacAuthEvent en1   Auth result for: 00:60:64:1e:e9:e4 Unsolicited  Auth
wlEvent: en1 en1 Link UP
AirPort: Link Up on en1
en1: BSSID changed to 00:60:64:1e:e9:e4
virtual bool IOHIDEventSystemUserClient::initWithTask(task*, void*, UInt32): 
Client task not privileged to open IOHIDSystem for mapping memory (e00002c1)
 
[OSBoot1]
[OSBoot2]
[OSBoot3]
[OSBootTheme]

 
Boot Complete
"#;

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }
}

struct LineByLine<'a> {
    blink_speed: Duration,
    last_blink: web_time::Instant,
    time_per_line: Duration,
    last_line_time: web_time::Instant,
    lines: StatefulList<&'a str>,
    current_line: usize,
    complete_text: &'a str,
    cursor_on: bool,
}
impl<'a> LineByLine<'a> {
    pub fn new(lines: &'a str, cursor_on: bool, complete_text: &'a str) -> Self {
        Self {
            blink_speed: Duration::from_millis(1000),
            last_blink: web_time::Instant::now(),
            time_per_line: Duration::from_millis(100),
            last_line_time: web_time::Instant::now(),
            lines: StatefulList::with_items(lines.lines().collect()),
            current_line: 0,
            cursor_on: cursor_on,
            complete_text,
        }
    }

    pub fn percentage_displayed(&self) -> f64 {
        let total = self.lines.items.len() as f64;
        if total == 0.0 {
            return 0.0;
        }
        (self.current_line as f64 / total).clamp(0.0, 1.0)
    }

    pub fn up_to_current_line(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Max(1)]).split(area);

        let mut list: Vec<ListItem> = self
            .lines
            .items
            .iter()
            .take(self.current_line)
            .map(|line| {
                let content = vec![text::Line::from(Span::raw(*line))];
                ListItem::new(content)
            })
            .collect();

        let percentage = self.percentage_displayed();
        if percentage >= 1.0 {
            list.push(ListItem::new(vec![text::Line::from(Span::raw(
                self.complete_text,
            ))]));
        }

        let text = List::new(list)
            .block(Block::new())
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_stateful_widget(text, chunks[0], &mut self.lines.state);

        let gauge = Gauge::default()
            .block(Block::new())
            .gauge_style(Style::default().fg(Color::Red).bg(Color::Black))
            .use_unicode(true)
            .ratio(percentage);
        frame.render_widget(gauge, chunks[1]);
    }

    pub fn update(&mut self, _elapsed: Duration) {
        let now = web_time::Instant::now();
        let e = now.duration_since(self.last_line_time);
        if e.as_millis() as u32 > self.time_per_line.as_millis() {
            self.current_line += 1;
            self.last_line_time = now;
        }

        let e = web_time::Instant::now().duration_since(self.last_blink);
        if e.as_millis() as u32 > self.blink_speed.as_millis() {
            self.cursor_on = !self.cursor_on;
            self.last_blink = web_time::Instant::now();
        }

        let list: Vec<ListItem> = self
            .lines
            .items
            .iter()
            .take(self.current_line)
            .map(|line| {
                let content = vec![text::Line::from(Span::raw(*line))];
                ListItem::new(content)
            })
            .collect();
        self.lines.state.select(if list.is_empty() {
            None
        } else {
            Some(list.len() - 1)
        });
    }
}

pub struct IntroState {
    should_exit: bool,
    line_by_line: LineByLine<'static>,
}

impl IntroState {
    pub fn new() -> Self {
        Self {
            should_exit: false,
            line_by_line: LineByLine::new(BOOT, true, "press any key to continue..."),
        }
    }
    pub fn create() -> Box<dyn StateActions> {
        Box::new(IntroState::new())
    }
}

impl StateActions for IntroState {
    fn should_exit_state(&self) -> Result<&str, StateMachineError> {
        if self.should_exit {
            Ok("default")
        } else {
            Ok("")
        }
    }
    fn update_state(&mut self, _elapsed: Duration) -> Result<(), StateMachineError> {
        self.line_by_line.update(_elapsed);
        Ok(())
    }

    fn render_state(&mut self, frame: &mut Frame) {
        let block = Block::bordered();
        frame.render_widget(block, frame.area());

        let remaining_area = render_logo_text(frame, frame.area());
        self.line_by_line.up_to_current_line(frame, remaining_area);
    }

    fn key_press(&mut self, _key: KeyCode) -> Result<(), StateMachineError> {
        self.should_exit = true;
        Ok(())
    }
}
