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
[F1]About [F3]View [F4]Edit [F5]Copy [F6]Move [F7]MkDir [F8]Del [^Q]Quit
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

## 키 바인딩

### 탐색

| 키 | 동작 |
|---|---|
| `↑` / `↓` | 커서 이동 |
| `PageUp` / `PageDown` | 20줄씩 이동 |
| `Home` / `End` | 목록 처음/끝 |
| `Enter` | 디렉토리 진입 |
| `Tab` | 패널 전환 |

### 파일 선택

| 키 | 동작 |
|---|---|
| `Space` / `Insert` | 파일 태그 토글 (★) |

다중 선택된 파일은 F5/F6/F8 작업의 대상이 됩니다. 선택이 없으면 커서 위치의 파일에 작업합니다.

### 파일 작업

| 키 | 동작 |
|---|---|
| `F3` | 파일 내용 보기 |
| `F4` | 편집 (`$EDITOR`, 미설정 시 `vi`) |
| `F5` | 복사 (현재 패널 → 반대 패널) |
| `F6` | 이동 |
| `F7` | 디렉토리 생성 |
| `F8` | 삭제 (확인 후 실행) |
| `F1` | About |
| `Ctrl+Q` | 종료 |

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

## 개발

```bash
cargo build       # 디버그 빌드
cargo run         # 실행
cargo clippy      # 린트
cargo fmt         # 포맷
```
