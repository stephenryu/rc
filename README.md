# rc — Rust Commander

Norton Commander 스타일의 듀얼 패널 터미널 파일 매니저. Rust로 작성.

```
┌─/home/user──────────────────┐┌─/tmp────────────────────────┐
│▶ / ..          <DIR>        ││  / Documents   <DIR>        │
│  / Downloads   <DIR>        ││  / Pictures    <DIR>        │
│  ★ report.pdf    1.2M       ││    notes.txt       4.1K     │
│    main.rs      12.3K       ││    build.log      89.0K     │
└─────────────────────────────┘└─────────────────────────────┘
 /home/user → /tmp
[F1]About [F2]Rename [F3]View [F4]Edit [F5]Copy [F6]Move [F7]MkDir [F8]Del [^Q]Quit
```

## 설치 및 빌드

```bash
git clone <repo>
cd rc
cargo build --release
./target/release/rc
```

## 사용법

```bash
rc                        # 현재 디렉토리로 시작
rc /path/to/dir           # 왼쪽 패널 지정
rc /left/dir /right/dir   # 양쪽 패널 각각 지정
rc --help
rc --version
```

오른쪽 패널은 마지막으로 열었던 경로를 `~/.rc/right`에 저장해 두고, 다음 실행 시 자동으로 복원합니다. 인자로 경로를 지정하면 저장된 값보다 인자가 우선합니다.

## 키 바인딩

### 탐색

| 키 | 동작 |
|---|---|
| `↑` / `↓` | 커서 이동 |
| `←` / `→` | 왼쪽/오른쪽 패널 활성화 |
| `PageUp` / `PageDown` | 20줄씩 이동 |
| `Home` / `End` | 목록 처음/끝 |
| `Tab` | 패널 전환 |
| `영문자 키` | 해당 글자로 시작하는 항목으로 이동 (반복 입력 시 순환) |

### 파일 열기

| 키 | 동작 |
|---|---|
| `Enter` | 디렉토리 진입 / 실행 파일 실행 |

- **디렉토리**: 진입. 왼쪽 패널은 프로세스 작업 디렉토리도 함께 변경됩니다.
- **실행 파일**: TUI를 잠깐 벗어나 터미널에서 직접 실행하고, 종료 후 복귀합니다.
  - Unix: 실행 권한(`chmod +x`)이 있는 파일
  - Windows: `.exe` `.bat` `.cmd` `.com`

### 파일 선택

| 키 | 동작 |
|---|---|
| `Space` / `Insert` | 파일 태그 토글 (★) |

다중 선택된 파일은 F5/F6/F8 작업의 대상이 됩니다. 선택이 없으면 커서 위치의 파일에 작업합니다.

### 파일 작업

| 키 | 동작 |
|---|---|
| `F1` | About |
| `F2` | 이름 변경 (현재 이름이 미리 채워짐) |
| `F3` | 파일 내용 보기 (언어별 구문 하이라이팅) |
| `F4` | 편집 (`$EDITOR`, 미설정 시 `vi`) |
| `F5` | 복사 (현재 패널 → 반대 패널) |
| `F6` | 이동 |
| `F7` | 디렉토리 생성 |
| `F8` | 삭제 (확인 후 실행) |
| `Ctrl+Q` | 종료 |

### 뷰어 (F3)

| 키 | 동작 |
|---|---|
| `↑` / `↓` | 한 줄씩 스크롤 |
| `PageUp` / `PageDown` | 페이지 스크롤 |
| `Home` / `End` | 처음/끝으로 이동 |
| `Esc` / `F3` / `q` | 닫기 |

파일 확장자를 자동 감지해 Rust, Python, JavaScript, TOML 등 수백 가지 언어의 구문 하이라이팅을 지원합니다.

### 복사/이동 충돌 처리

같은 이름의 파일이 대상에 존재하면 선택 다이얼로그가 표시됩니다.

| 키 | 동작 |
|---|---|
| `O` | 이 파일 덮어쓰기 |
| `A` | 이후 모든 충돌 파일 덮어쓰기 |
| `S` | 이 파일 건너뛰기 |
| `N` | 이후 모든 충돌 파일 건너뛰기 |
| `R` | 자동 이름 변경 (`file_copy.txt`, `file_copy2.txt`, …) |
| `Esc` | 나머지 작업 취소 |

## 상태 파일

| 파일 | 내용 |
|---|---|
| `~/.rc/right` | 오른쪽 패널의 마지막 경로 (시작 시 자동 복원) |

## 개발

```bash
cargo build       # 디버그 빌드
cargo run         # 실행
cargo clippy      # 린트
cargo fmt         # 포맷
cargo test        # 테스트
```

## 크로스 컴파일 (Linux x64)

Docker Desktop이 실행 중인 상태에서 `cross`로 Linux 바이너리를 빌드할 수 있습니다.

```bash
# 최초 1회 설정
cargo install cross
rustup target add x86_64-unknown-linux-gnu

# 빌드
cross build --target x86_64-unknown-linux-gnu --release
```

결과물: `target/x86_64-unknown-linux-gnu/release/rc`
