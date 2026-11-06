# TRAPDOOR 詳細設計書 v0.4

映像ポン出しソフト。Mitti のキューリスト、PVP3 のクリップバンク、Grass Valley T3 の「次を頭出しして待つ」運用を、1つの画面でできるようにする。

- 作成日：2026-11-06
- 状態：承認済み・M1 実装中
- 名前：TRAPDOOR（舞台の床の仕掛け扉。映像を「ポン」と出すイメージ）
- 共有版の設計書（Claude Docs）と同じ内容。こちらはリポ内の控え

---

## 1. 要件定義

### 1.1 目的
- 講演会・式典・ライブで、映像を「確実に・頭から・狙った瞬間に」出す
- Mac 専用の PVP3 / Mitti に頼らず、**Windows 機1台で**同じ使い勝手にする
- 本番中に迷わない画面（残り時間・次に出るもの・今出ているものが一目で分かる）

### 1.2 参考ソフトから取り入れるもの

| 参考 | 取り入れる操作 | 本ソフトでの形 |
|---|---|---|
| Mitti | キューリストを上から順に GO（スペースキー）で進める。キューごとに「頭で一時停止／尻で一時停止」「フェードイン・アウト」「ループ」「終わったら指定キューへ」。PANIC（フェードアウトして止める）。残り10/20/30秒へ飛ぶ（Goto 10/20/30）。再生速度 | **キューリスト画面**（メイン） |
| PVP3 | 素材をサムネのボタン並びで持ち、押した瞬間にトランジション付きで切り替える。SDI・NDI 出力、Key/Fill 出力 | **クリップバンク**（F1〜F12＋数字キーで即出し） |
| GV T3 | 次の素材を頭出しして PREVIEW に出して待ち、TAKE で PROGRAM に出す。プレイリスト運用。外部機器からの制御（AMP 等）、タイムコード追従 | **PREVIEW／PROGRAM の2画面モニター**、STANDBY→TAKE の流れ |

根拠（公式・報道資料）は末尾「参考資料」に記載。

### 1.3 機能一覧（段階分け）

**M1（最初に作る範囲・現場で使える最小構成）**
1. キューリスト：番号・サムネ・名前・尺・IN/OUT・終了時の動作・フェード・音量・メモ・色ラベル
2. 終了時の動作：停止して黒／最終フレームで止める／次のキューへ自動／ループ（回数指定可）／指定キューへ
3. 開始時の動作：すぐ再生／頭のフレームで一時停止して待つ
4. 操作：GO（スペース）、STANDBY 移動（↑↓）、TAKE、一時停止、STOP、PANIC（Esc 2回、フェードアウトして黒）、Goto −30/−20/−10秒
5. PREVIEW（次）／PROGRAM（今）の2モニター表示、大きな残り時間（REMAIN）表示、進行バー
6. クリップバンク：4×4〜8×4 のボタン。F1〜F12 で即出し。押したら設定トランジション（カット／ディゾルブ）で PROGRAM に出る
7. 画面出力：指定したディスプレイにフルスクリーン（枠なし・最前面・マウス非表示）。出力の黒／テストパターン切替
8. 再生エンジン：A/B 2台の再生機を交互に使い、**次のキューは事前に読み込んで頭出し済み**にする（つなぎ目で黒が挟まらない）。ディゾルブは2台を重ねて合成
9. 音声：出力機器の選択、キューごとの音量、マスター音量とメーター、ミュート
10. ショーファイル保存（.trapdoor、中身は JSON）、自動保存、素材の場所が変わっても探し直せる（リンク切れ検出）
11. 本番前チェック：全キューの読み込みテスト（再生できない素材・コーデック・解像度/fps違いを警告）
12. 外部制御：OSC、HTTP/WebSocket（Bitfocus Companion・Stream Deck から押せる）
13. 誤操作防止：本番ロック（ロック中は編集不可・GO と PANIC だけ有効）、STOP は長押し

**M2（次）**
- NDI 出力（映像＋音声、アルファ付き）
- Blackmagic DeckLink の SDI 出力（Fill／Key の2系統）
- タイムコード：LTC 入力に追従して再生、キューの開始時刻をタイムコードで予約
- MIDI 入力（ノート・CC・MSC）
- ATEM 連携：ATEM で指定入力が選ばれたら自動で GO

**M3（その後）**
- 出力を複数（例：正面＋サイド）、レイヤー（ロゴ重ね）
- 台形補正・エッジブレンド
- HyperDeck のふり（ATEM から HyperDeck として操作される）、AMP（T2/T3 系の外部制御）
- 字幕（SRT）重ね

### 1.4 動作環境
- **Windows 10/11 64bit を主対象**（Mac を持っていないため、実機確認できるのは Windows）
- macOS は GitHub Actions の Mac ランナーでビルドだけ通す（M2 以降で正式対応）
- 想定 PC：GPU 付き（NVIDIA / AMD / Intel 内蔵どれでも）、メモリ 16GB 以上

### 1.5 対応素材
- 動画：MP4 / MOV / MXF / AVI。H.264・H.265・ProRes・DNxHD/HR・HAP・Motion JPEG・VP9・AV1 など（FFmpeg が読めるもの）
- 静止画：PNG / JPEG / TIFF / BMP（表示秒数指定 or 無期限）
- 音声だけ：WAV / MP3 / AAC / FLAC
- 目標：1080p59.94 を1出力でコマ落ちなし。4K はハードウェアデコード前提で M2 で検証

### 1.6 品質目標
- GO を押してから PROGRAM に映るまで：**2フレーム以内**（頭出し済みのキュー）
- 1時間連続再生でコマ落ち 0（統計を画面に常時表示）
- 落ちても出力窓は黒で残る（エンジンと操作画面を分け、操作画面が落ちても再生は止まらない）

---

## 2. 技術選定

### 2.1 一番大事な判断
ポン出しソフトの心臓は「再生エンジン」。**ブラウザの `<video>` では作らない**。理由：
- ProRes・HAP・DNxHR などが再生できない
- フレーム単位の頭出し・2台同期・コマ落ちの監視ができない
- SDI・NDI に出せない

そのため「再生エンジンはネイティブ（Rust＋libmpv）」「操作画面は作りやすい技術」に分ける。

### 2.2 候補比較

| | A. Tauri＋Rust＋libmpv | B. Qt（C++/QML）＋libmpv | C. C#（.NET 10）＋WinUI 3＋libmpv/LibVLC |
|---|---|---|---|
| 動作の軽さ | ◎ エンジンはネイティブ。操作画面は Windows 標準の WebView2 | ◎ 全部ネイティブ | ○ ネイティブ。.NET の実行環境が乗る |
| 配布サイズ | ○ 約30〜60MB（libmpv＋FFmpeg 同梱が大半） | △ 約80〜150MB（Qt 本体が重い） | △ 約80MB〜（.NET 同梱時） |
| 作りやすさ | ◎ 操作画面は React で速く作れる。エンジンは Rust | △ C++ と QML の両方、ビルドが重い | ○ Windows だけなら作りやすい |
| 対応OS | ◎ Win / Mac（Linux も可） | ◎ Win / Mac / Linux | × Windows のみ |
| 将来性 | ◎ Tauri 2 は活発（2026/10 も更新） | ○ 安定。商用利用はライセンス注意 | ○ .NET 10 は 2028/11 までサポート |
| AIでの書きやすさ | ◎ React/TS と Rust はどちらも得意 | △ QML の書き例が少なめ | ○ |
| SDI/NDI | ◎ どちらも C の SDK を Rust から呼べる | ◎ C++ から直接 | ○ 包み直しが要る |

### 2.3 おすすめ：**A. Tauri 2 ＋ Rust ＋ libmpv**
- 理由1：再生の重い部分はネイティブで確実、操作画面は React で細かい見た目まで作り込める
- 理由2：後で Mac 版も同じコードで出せる（GitHub Actions でビルド）
- 理由3：LEQVIO TIMER で Tauri 化を進めている流れと揃う
- 将来：自作 GPU エンジン（onchip）が育ったら、libmpv 部分だけ差し替えられる作り（エンジンを「再生機の箱」として抽象化しておく）

### 2.4 使う技術と版（2026-11-06 時点、公式で確認）

| 種類 | 名前 | 版 | 用途 | 根拠 |
|---|---|---|---|---|
| アプリの枠 | Tauri（Rust クレート） | 2.12.3（2026-10-10） | 窓・ファイル・配布 | https://v2.tauri.app/release/ |
| 〃 | @tauri-apps/cli | 2.12.2（2026-10-10） | ビルド | 同上 |
| 言語 | Rust | 1.99.1（2026-10-15） | エンジン | https://blog.rust-lang.org/ |
| 再生 | mpv / libmpv | 0.41.0（2025-12-21、最新安定版） | 動画の読み込み・デコード・描画 | https://github.com/mpv-player/mpv/releases |
| デコード | FFmpeg | 9.0.2（2026-09-18） ※libmpv の配布ビルドに同梱される版に従う | 各コーデック | https://ffmpeg.org/download.html |
| 操作画面 | React | 19.3.1（2026-10-14） | 画面部品 | https://github.com/facebook/react/releases |
| 〃 | Vite | 8.3.5 | 開発・ビルド | https://github.com/vitejs/vite/releases |
| 〃 | Tailwind CSS | 4.3.3 | 見た目 | https://github.com/tailwindlabs/tailwindcss/releases |
| 〃 | TypeScript | 設計承認後に最新安定版を確認して記入 | | |
| M2 | NDI SDK | 6.3.1 | NDI 出力 | https://docs.ndi.video/all/docs/sdk/release-notes |
| M2 | Blackmagic Desktop Video SDK | 16.0（2026-04-08）／ドライバー最新 16.5（2026-10-30） | SDI 出力 | https://www.blackmagicdesign.com/developer/products/capture-and-playback/sdk-and-software |

注意点
- **mpv のライセンス**：標準ビルドは GPL。自作OSSとして GitHub で公開するなら問題なし。将来売る場合は LGPL 版の libmpv（`-Dgpl=false` でビルド）に切り替える必要あり
- NDI SDK は無料だが利用規約あり（6.2.1 で規約更新）。M2 着手時に再確認
- ASIO（低遅延の音声ドライバー）は mpv が非対応。M1 は WASAPI（Windows 標準）。ASIO が必要なら M2 で音声だけ自前処理に切り替える

---

## 3. 全体の作り（構成図）

```
┌──────────── 操作画面（WebView2 / React） ────────────┐
│ キューリスト / クリップバンク / モニター / 設定        │
└───────▲ 状態（30回/秒） ───────────── 命令 ▼────────┘
        │        Tauri の IPC（内部の連絡路）            
┌───────┴──────────── エンジン（Rust） ─────────────────┐
│  ShowController … キューの進行・GO/TAKE/PANIC の判断   │
│  Deck A ─┐  libmpv（描画は自前の OpenGL へ）          │
│  Deck B ─┼→ Compositor（ディゾルブ合成・黒・テスト画） │
│  Bank用 ─┘        │                                   │
│                   ├→ 出力窓（指定ディスプレイ全画面）  │
│                   ├→ プレビュー縮小画像 → 操作画面へ   │
│                   └→ (M2) NDI / DeckLink へ           │
│  Remote … OSC(UDP 53000) / HTTP+WebSocket(8780)       │
│  Clock … 出力の垂直同期を基準にした時計               │
└───────────────────────────────────────────────────────┘
```

### 3.1 大事な仕組み
1. **2台交互（A/B デッキ）**：PROGRAM で A が再生中、STANDBY のキューは B に読み込み済み・頭のフレームで停止状態。GO で B を出し、A を次の次の準備に回す
2. **描画は自前**：libmpv の描画機能（render API）で、各デッキの映像を自分の OpenGL のテクスチャに描かせる。それを合成して出力窓に表示。これでディゾルブも、M2 の NDI・SDI 出力（画像を読み出して送る）もできる
3. **プレビューの送り方**：PREVIEW/PROGRAM モニターは、エンジンが 480×270 の縮小画像を 15〜30回/秒で操作画面へ送る（専用の内部プロトコル `swframe://`）。本物の出力とは別経路なので、操作画面が重くなっても出力は止まらない
4. **時計**：残り時間は「再生位置」ではなく「出力に出たフレーム数」から計算（表示とズレない）
5. **フレームレート**：プロジェクトの基準 fps（既定 59.94）を持ち、違う fps の素材は本番前チェックで警告

---

## 4. 画面構成

### 4.1 メイン画面（1920×1080 想定、最小 1440×900）

```
┌ ヘッダー：ロゴ / ショー名 / 時計 / 出力状態(OUT1・NDI・SDI) / コマ落ち / ロック ┐
├──────────────────────┬──────────────────────────────┤
│ キューリスト（左 58%）│ PREVIEW(次)     PROGRAM(今)   │
│  # サムネ 名前 尺     │ ┌────────┐ ┌────────┐ │
│  IN/OUT 終了動作 音量 │ └────────┘ └────────┘ │
│  ▶再生中は赤、         │  REMAIN 00:01:23.12（特大）    │
│  STANDBYは黄の枠      │  進行バー（IN/OUT・フェード位置）│
│                      │  [GO] [TAKE] [PAUSE] [STOP]    │
│                      │  [-30][-20][-10] [LOOP] [PANIC]│
├──────────────────────┴──────────────────────────────┤
│ 下段タブ：クリップバンク / キュー設定 / メディア / ログ    │ 音声メーター │
└──────────────────────────────────────────────────────┘
```

### 4.2 画面一覧
| 画面 | 中身 |
|---|---|
| メイン | 上の通り。本番はここだけで完結 |
| キュー設定（下段タブ） | 選んだキューの IN/OUT、開始・終了時の動作、フェード秒数、ループ回数、音量、色、メモ、ホットキー |
| クリップバンク（下段タブ） | 8×2 のボタン（F1〜F12＋Shift）。各ボタンにサムネ・名前・尺・キー |
| メディア（下段タブ） | フォルダから素材を読み込み、情報（コーデック・解像度・fps・音声ch）表示、ドラッグでキューへ |
| 出力設定（窓） | 出力先ディスプレイ、解像度、拡大方法（収める/埋める/そのまま）、背景色、マウス非表示 |
| 本番前チェック（窓） | 全キューの読み込み結果一覧。NG は赤 |
| 外部制御（窓） | OSC/HTTP の受付ポート、命令一覧、受信ログ |
| 出力窓 | 映像だけ。何も出していない時は黒（設定でロゴ静止画も可） |

### 4.3 色の意味（現場の慣習に合わせる）
- 赤 `#E5383B`：PROGRAM（今出ている）
- 緑 `#2FB344`：PREVIEW／準備完了
- 黄 `#F2A900`：STANDBY（次に GO で出るもの）、残り10秒以下の警告
- 明るい灰 `#D0D5DA`：選択中・編集中（色は増やさない）
- 背景 `#0E0F10`、面 `#17181A`、線 `#2A2C2F`、文字 `#D9DCDF`、弱い文字 `#8A9096`
- 文字：数字は Space Mono（桁が揃う）、日本語は Noto Sans JP（自作ツールなのでブランド規定は外して色を使う）

### 4.4 キー操作（既定）
| キー | 動作 |
|---|---|
| Space | GO（STANDBY を出す） |
| Enter | TAKE（選択中のキューをすぐ出す） |
| ↑ / ↓ | STANDBY を上下に移動 |
| P | 一時停止／再開 |
| Esc ×2（0.5秒以内） | PANIC（フェードアウトして黒） |
| S 長押し 0.5秒 | STOP（すぐ黒） |
| 1 / 2 / 3 | 残り 10 / 20 / 30 秒へ |
| L | ループ切替 |
| F1〜F12（Shift 併用で 24個） | クリップバンク |
| Ctrl+L | 本番ロック |
| B | 出力を黒（もう一度で戻す） |

---

## 5. 部品の分け方（操作画面 React）

```
App
├─ HeaderBar          … ロゴ、ショー名、時計、OutputStatus、DropCounter、LockToggle
├─ MainSplit
│  ├─ CueList         … 仮想スクロール（1000キューでも軽い）
│  │   └─ CueRow      … 番号・サムネ・名前・尺・終了動作アイコン・音量・状態色
│  └─ PlayoutPanel
│      ├─ MonitorPair … Monitor(PREVIEW) / Monitor(PROGRAM)
│      ├─ RemainClock … 特大残り時間、10秒以下で黄、0で赤点滅
│      ├─ ProgressBar … IN/OUT・フェード区間・現在位置・クリックでシーク（ロック中無効）
│      └─ Transport   … GO / TAKE / PAUSE / STOP / Goto / LOOP / PANIC
├─ BottomTabs
│  ├─ ClipBank → BankButton
│  ├─ CueInspector
│  ├─ MediaBrowser
│  └─ EventLog
├─ AudioMeters        … マスター L/R（多ch時は自動で本数増）
└─ Dialogs            … OutputSettings / PreflightCheck / RemoteSettings
```

---

## 6. 状態管理

- **正しい状態はエンジン（Rust）だけが持つ**。操作画面は「表示用のコピー」を受け取るだけ（二重管理しない）
- 操作画面は Zustand（小さな状態管理ライブラリ）に「エンジンから来た最新状態」を入れて描画
- エンジン → 画面：`playout`（30回/秒：位置・残り・状態）、`show`（変更時のみ：キュー一覧）、`meters`（30回/秒）、`log`
- 画面 → エンジン：命令だけ送る（`go`, `take(cueId)`, `panic`, `updateCue(id, patch)` など）。OSC・HTTP から来る命令も同じ入口に入る
- 元に戻す（Undo）：キュー編集だけ対象。エンジン側で履歴を持つ

### 6.1 再生の状態の流れ（PROGRAM 側）
```
EMPTY ─load→ CUED(頭で停止) ─GO→ PLAYING ─OUT点→ 終了時動作
  ▲                               │  ├ hold  → HOLD(最終フレーム)
  │                               │  ├ black → EMPTY
  └──────── STOP / PANIC完了 ─────┘  ├ next  → 次キューを PLAYING
                                     ├ loop  → PLAYING(IN点へ)
                                     └ goto  → 指定キューを PLAYING
PLAYING ⇄ PAUSED、PLAYING ─PANIC→ FADING_OUT → EMPTY
```

---

## 7. データ構造（ショーファイル .trapdoor ＝ JSON）

```jsonc
{
  "format": "trapdoor",
  "version": 1,
  "show": {
    "name": "サンプル式典（架空）",
    "frameRate": "59.94",
    "output": { "display": 2, "width": 1920, "height": 1080, "fit": "fit", "background": "#000000" },
    "audio": { "device": "default", "masterDb": 0 },
    "transitionDefault": { "type": "cut", "durationMs": 0 }
  },
  "media": [
    { "id": "m_01", "path": "D:/show/opening.mov", "relPath": "media/opening.mov",
      "hash": "sha1:…", "codec": "prores", "width": 1920, "height": 1080,
      "fps": "59.94", "durationFrames": 5394, "audioChannels": 2 }
  ],
  "cues": [
    { "id": "c_01", "number": "1", "name": "オープニング", "mediaId": "m_01",
      "inFrame": 0, "outFrame": 5394,
      "start": "play",            // play | pauseAtStart
      "end": "hold",              // black | hold | next | loop | goto
      "loopCount": 0, "gotoCueId": null,
      "fadeInMs": 0, "fadeOutMs": 500,
      "transition": { "type": "dissolve", "durationMs": 500 },
      "volumeDb": 0, "speed": 1.0,
      "color": "red", "note": "司会のきっかけで GO", "hotkey": null }
  ],
  "bank": [ { "slot": "F1", "cueId": "c_05" } ],
  "remote": { "oscPort": 53000, "httpPort": 8780 }
}
```
- 時間は**すべてフレーム数で持つ**（ミリ秒の誤差を出さない）。表示時だけ時:分:秒.フレームに変換
- 素材は「絶対パス」「ショーファイルからの相対パス」「中身の指紋（先頭と末尾 1MB のハッシュ）」の3つで探す → USB で別PCに持って行っても見つかる
- 自動保存：変更から 2秒後に `.trapdoor.autosave`、起動時に新しい方を提案

### 7.1 外部制御（OSC の例）
| アドレス | 動作 |
|---|---|
| `/trapdoor/go` | GO |
| `/trapdoor/take <番号>` | 指定キューを出す |
| `/trapdoor/standby <番号>` | STANDBY を指定 |
| `/trapdoor/pause` `/trapdoor/stop` `/trapdoor/panic` | 各操作 |
| `/trapdoor/goto <秒>` | 残り◯秒へ |
| `/trapdoor/bank <F1〜>` | クリップバンク |
送り返し（フィードバック）：`/trapdoor/state/remain`（"00:01:23"）、`/trapdoor/state/program`（キュー名）など。HTTP は同じ命令を `POST /api/go` などで受ける。

---

## 8. フォルダ構成

```
trapdoor/
├─ src-tauri/                 … Rust 側
│  ├─ src/
│  │  ├─ main.rs              … 起動・窓
│  │  ├─ ipc.rs               … 画面との命令/状態のやりとり
│  │  ├─ show/                … ショーファイル・キュー・Undo
│  │  ├─ engine/
│  │  │  ├─ deck.rs           … libmpv 1台分（読み込み・頭出し・再生）
│  │  │  ├─ compositor.rs     … 合成（ディゾルブ・黒・テスト画）
│  │  │  ├─ output_window.rs  … 出力窓（全画面・最前面）
│  │  │  ├─ preview.rs        … 縮小画像を作って画面へ送る
│  │  │  ├─ clock.rs          … フレーム基準の時計
│  │  │  └─ controller.rs     … GO/TAKE/PANIC の判断と状態遷移
│  │  ├─ remote/ (osc.rs, http.rs)
│  │  └─ preflight.rs         … 本番前チェック
│  ├─ vendor/libmpv/          … 配布用 DLL（CI で取得）
│  └─ tauri.conf.json
├─ src/                       … 操作画面（React＋TS＋Tailwind）
│  ├─ components/（上の部品ごと）
│  ├─ store/ engine.ts        … Zustand
│  ├─ lib/ ipc.ts, timecode.ts, keys.ts
│  └─ styles/ tokens.css      … 色・文字の定義
├─ docs/ design.md, design/main.svg
├─ samples/                   … 架空のテスト素材（カラーバー・カウント動画を FFmpeg で生成）
└─ .github/workflows/         … Windows ビルド（exe 作成）、Mac ビルド（確認のみ）、版番号でリリース
```

---

## 9. 開発の進め方（M1 の中身）
1. エンジンだけで動く試作：libmpv 2台＋出力窓で「頭出し → GO → ディゾルブ」が 59.94p で途切れないか確認（ここが一番の山）
2. 操作画面（SVG デザイン通り）＋状態のやりとり
3. ショーファイル保存・本番前チェック・キー操作・ロック
4. OSC/HTTP、Companion から押せることを確認
5. GitHub Actions で exe を作り、main に入れたら自動リリース

試験用の素材は FFmpeg で生成する架空のもの（カラーバー＋フレーム番号入りカウントダウン、59.94/29.97/25 の3種）。

## 10. 決まったこと（2026-11-06）
- 名前：TRAPDOOR
- 画面の言葉：英語が基本（日本語に切替可、src/i18n）
- 見た目：無骨な業務機材風（角を丸めない・色は赤緑黄の3つだけ・PREVIEW/PROGRAM に色の帯）
- NDI 出力：M2（おまかせ）
- 出力解像度：既定 1920×1080、プリセットは HD / WUXGA / UHD 4K / DCI 4K / UHD 8K / DCI 8K、カスタム可（最大 8192×4320、偶数に丸め）。画面の左上に等倍で置き、画面より大きい時は縦横比を保って縮小
- 波形表示：進行バー（48px）、キュー一覧の各行、クリップバンクの各ボタン

## 11. キューごとの調整（非破壊）
素材ファイルは書き換えず、再生時だけかける。

| 分類 | 項目 | 範囲・既定 |
|---|---|---|
| 時間 | IN/OUT、速度 | 速度 0.25〜2.00× |
| 音 | ゲイン | −60〜+12 dB |
| 音 | 音量の自動そろえ | 目標 −23 LUFS（変更可）。読み込み時に測定 |
| 音 | フェード入/出、ミュート | 0〜10 秒（画も黒から/黒へフェード） |
| 音 | チャンネル割り当て・パン | 素材の ch → 出力 L/R |
| 画 | 拡大方法・拡大率・位置 | Fit/Fill/Stretch/1:1、25〜400% |
| 画 | 切り取り、不透明度、反転 | |
| 画 | 明るさ・コントラスト・彩度 | −100〜+100 |

## 12. M1 実装で割り切ったこと（正直な現状）
- ディゾルブは「黒を挟む」形（前の映像を半分の時間で黒へ→次を半分の時間で黒から）。2 つの映像が重なる本物のクロスフェードは、自前合成（libmpv の描画機能）を入れる M2 で対応
- 不透明度・フェードは「黒の上に重ねた明るさ」で表現（mpv の brightness を下げる）。見た目は黒へのフェードと同じだが、厳密な線形ではない
- PREVIEW/PROGRAM モニターは本物の映像ではなく、読み込み時に作るサムネイル（PROGRAM は 1 秒ごとのコマ）。本物の映像の縮小表示は M2
- 音声メーターは M1 では出ない（mpv から音の大きさを取り出す仕組みが要る）。M2
- 出力窓は Windows のみ。Mac/Linux は開発用に mpv の窓が 2 つ出る

## 13. 同梱するもの（Windows）
- libmpv-2.dll（shinchiro/mpv-winbuild-cmake の最新ビルド、GPL）
- ffmpeg.exe / ffprobe.exe（BtbN/FFmpeg-Builds の最新リリース系列、GPL）。素材の情報・サムネイル・波形・ラウドネス測定に使う
- そのためアプリ全体のライセンスは GPL-3.0-or-later。配布サイズは設計時の見込み（30〜60MB）より大きく、ffmpeg 分で 150〜250MB 程度になる

---

## 参考資料
- PVP3：https://renewedvision.com/?p=207822 、https://renewedvision.com/blog/?p=2728
- Mitti：https://www.rentex.com/?p=13928 、https://marketplace.elgato.com/@imimothq 、https://github.com/bitfocus/companion-module-imimot-mitti
- GV T3：https://www.sportsvideo.org/2024/04/17/nab-2024-grass-valley-showcases-next-generation-t3-series-4k-hd-digital-recording-and-playback-for-venues-and-stadiums/
- 読めていない部分：PVP3・Mitti・T3 の公式マニュアル本体は読んでいない（製品紹介・報道資料・連携モジュールの説明から機能を拾った）。T3 の画面の細かい操作手順は未確認
