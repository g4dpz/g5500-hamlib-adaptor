# Kiro Steering Files - Quick Index

## 📋 File Guide

### Start Here
- **README.md** - Overview of all steering files, quick reference, and development workflow

### Project Understanding
- **PROJECT_OVERVIEW.md** - What the project does, hardware, features, and structure
- **ANALYSIS_SUMMARY.txt** - Comprehensive analysis of the entire project

### Development
- **CODE_PATTERNS.md** - Coding conventions, patterns, and best practices
- **TECH_STACK.md** - Dependencies, build configuration, and memory constraints

### Building & Deployment
- **BUILD_DEPLOYMENT.md** - Build system, flashing methods, and deployment procedures

### Hardware
- **HARDWARE_ARCHITECTURE.md** - Pin assignments, specifications, and hardware details

---

## 🎯 Quick Navigation by Task

### "I want to understand the project"
1. Read: PROJECT_OVERVIEW.md
2. Read: ANALYSIS_SUMMARY.txt
3. Skim: README.md

### "I want to write code"
1. Read: CODE_PATTERNS.md
2. Reference: TECH_STACK.md (for memory constraints)
3. Reference: HARDWARE_ARCHITECTURE.md (for pin assignments)

### "I want to build and flash"
1. Read: BUILD_DEPLOYMENT.md
2. Reference: README.md (quick commands)

### "I need to debug hardware"
1. Read: HARDWARE_ARCHITECTURE.md
2. Reference: README.md (pin assignments)

### "I'm new to the project"
1. Read: README.md
2. Read: PROJECT_OVERVIEW.md
3. Read: CODE_PATTERNS.md
4. Read: BUILD_DEPLOYMENT.md
5. Reference: HARDWARE_ARCHITECTURE.md as needed

---

## 📊 File Statistics

| File | Lines | Purpose |
|------|-------|---------|
| README.md | 261 | Overview and quick reference |
| PROJECT_OVERVIEW.md | 54 | Project summary |
| TECH_STACK.md | 146 | Dependencies and build config |
| CODE_PATTERNS.md | 324 | Coding conventions |
| BUILD_DEPLOYMENT.md | 371 | Build and deployment |
| HARDWARE_ARCHITECTURE.md | 312 | Hardware specifications |
| ANALYSIS_SUMMARY.txt | 481 | Comprehensive analysis |
| **TOTAL** | **1,949** | **Complete project documentation** |

---

## 🔑 Key Information at a Glance

### Project
- **Name**: G-5500 Hamlib Adaptor
- **Language**: Rust (no_std, embedded)
- **Framework**: Embassy (async runtime)
- **Hardware**: RP2040 (Cortex-M0+) with W5500 Ethernet
- **License**: BSD 3-clause

### Architecture
- **Concurrency**: Task-based (Embassy executor)
- **Protocol**: HamLib rotctld (partial implementation)
- **Network**: TCP/IP with DHCP
- **Hardware**: SPI (W5500), ADC (position), GPIO (relays/LEDs)

### Constraints
- **RAM**: 264KB total (126KB static, 138KB available)
- **Flash**: 2MB (244KB firmware, 1.8MB available)
- **Watchdog**: 8.3s timeout
- **DHCP**: 5s timeout
- **Sockets**: 4 concurrent connections

### Key Commands
```bash
cargo build --release    # Build optimized firmware
cargo run                # Flash and run
cargo bloat --release    # Analyze binary size
telnet <ip> 4533         # Connect to device
```

### Pin Assignments (Quick)
- **Control**: GPIO 2-5 (Az CW/CCW, El UP/DN)
- **Position**: GPIO 26-27 (Az/El ADC)
- **LEDs**: GPIO 25 (system), GPIO 15 (sockets)
- **Ethernet**: GPIO 16-21 (SPI + control)

---

## 📚 Documentation Structure

```
.kiro/
├── INDEX.md                    ← You are here
├── README.md                   ← Start here
├── PROJECT_OVERVIEW.md         ← What is this?
├── TECH_STACK.md              ← What technologies?
├── CODE_PATTERNS.md           ← How to code?
├── BUILD_DEPLOYMENT.md        ← How to build?
├── HARDWARE_ARCHITECTURE.md   ← What hardware?
└── ANALYSIS_SUMMARY.txt       ← Full analysis
```

---

## 🚀 Getting Started Checklist

- [ ] Read README.md
- [ ] Read PROJECT_OVERVIEW.md
- [ ] Install Rust 1.85.1+
- [ ] Install probe-rs: `cargo install probe-rs --features=cli`
- [ ] Read CODE_PATTERNS.md
- [ ] Read BUILD_DEPLOYMENT.md
- [ ] Build firmware: `cd firmware && cargo build --release`
- [ ] Flash device: `cd firmware && cargo run`
- [ ] Test with telnet: `telnet <device-ip> 4533`

---

## 💡 Common Questions

**Q: Where do I find pin assignments?**
A: HARDWARE_ARCHITECTURE.md - Pin Assignments section

**Q: How do I build the firmware?**
A: BUILD_DEPLOYMENT.md - Build Profiles section

**Q: What are the coding conventions?**
A: CODE_PATTERNS.md - Naming Conventions section

**Q: How much RAM is available?**
A: TECH_STACK.md - Memory Layout section (138KB available)

**Q: How do I flash the device?**
A: BUILD_DEPLOYMENT.md - Flashing Methods section

**Q: What's the project architecture?**
A: CODE_PATTERNS.md - Architecture Overview section

**Q: How do I debug issues?**
A: BUILD_DEPLOYMENT.md - Troubleshooting section

**Q: What hardware is used?**
A: HARDWARE_ARCHITECTURE.md - RP2040 Microcontroller section

---

## 📖 Reading Order by Role

### Embedded Systems Developer
1. PROJECT_OVERVIEW.md
2. TECH_STACK.md
3. CODE_PATTERNS.md
4. HARDWARE_ARCHITECTURE.md
5. BUILD_DEPLOYMENT.md

### Hardware Engineer
1. PROJECT_OVERVIEW.md
2. HARDWARE_ARCHITECTURE.md
3. BUILD_DEPLOYMENT.md (for testing)

### DevOps / Build Engineer
1. BUILD_DEPLOYMENT.md
2. TECH_STACK.md
3. README.md (quick reference)

### Project Manager
1. PROJECT_OVERVIEW.md
2. ANALYSIS_SUMMARY.txt
3. README.md (quick reference)

### New Team Member
1. README.md
2. PROJECT_OVERVIEW.md
3. CODE_PATTERNS.md
4. BUILD_DEPLOYMENT.md
5. HARDWARE_ARCHITECTURE.md
6. TECH_STACK.md

---

## 🔗 Cross-References

### From CODE_PATTERNS.md
- See TECH_STACK.md for memory constraints
- See HARDWARE_ARCHITECTURE.md for pin assignments
- See BUILD_DEPLOYMENT.md for build configuration

### From BUILD_DEPLOYMENT.md
- See TECH_STACK.md for dependency versions
- See HARDWARE_ARCHITECTURE.md for memory layout
- See CODE_PATTERNS.md for optimization patterns

### From HARDWARE_ARCHITECTURE.md
- See CODE_PATTERNS.md for ADC sampling patterns
- See TECH_STACK.md for memory layout
- See BUILD_DEPLOYMENT.md for debugging

### From TECH_STACK.md
- See CODE_PATTERNS.md for usage patterns
- See BUILD_DEPLOYMENT.md for build profiles
- See HARDWARE_ARCHITECTURE.md for memory mapping

---

## 📝 Maintenance Notes

These steering files should be updated when:
- New coding patterns are established → Update CODE_PATTERNS.md
- Dependencies change → Update TECH_STACK.md
- Build process changes → Update BUILD_DEPLOYMENT.md
- Hardware changes → Update HARDWARE_ARCHITECTURE.md
- Project scope changes → Update PROJECT_OVERVIEW.md

---

## ✅ Verification Checklist

All steering files are present:
- [x] README.md (261 lines)
- [x] PROJECT_OVERVIEW.md (54 lines)
- [x] TECH_STACK.md (146 lines)
- [x] CODE_PATTERNS.md (324 lines)
- [x] BUILD_DEPLOYMENT.md (371 lines)
- [x] HARDWARE_ARCHITECTURE.md (312 lines)
- [x] ANALYSIS_SUMMARY.txt (481 lines)
- [x] INDEX.md (this file)

**Total Documentation**: 1,949+ lines of comprehensive project guidance

---

**Last Updated**: 2025
**Status**: Complete and Ready for Use
