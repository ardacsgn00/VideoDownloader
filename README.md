# Video İndirici

yt-dlp ve ffmpeg tabanlı, son derece hafif (~625 KB), hızlı, koyu temalı ve 3D dokunsal (tactile) arayüze sahip modern masaüstü video & ses indirme yöneticisi.

## Özellikler

- **Ultra Hafif:** Yalnızca ~625 KB boyutunda tek bir çalıştırılabilir dosya (`VideoIndirici.exe`).
- **Sade & Koyu Tema:** Abartılı efekt ve emoji içermeyen, düz renkler ve fiziksel hisli 3D dokunsal butonlarla tasarlanmış arayüz.
- **Tek Satırda Hızlı İndirme:** Pencerenin ortasında konumlandırılmış link yapıştırma ve indirme alanı.
- **Canlı İlerleme Takibi:** Gerçek zamanlı indirme yüzdesi, hız (MB/s), kalan süre (ETA) ve indirilen boyut göstergesi.
- **3 Farklı İndirme Modu:**
  - `[1] En Yüksek Kalite Video:` Orijinal çözünürlük, h264 + aac formatında MP4 (Varsayılan).
  - `[3] 1080p Video:` 1080p sınırı ile dengeli dosya boyutu ve yüksek görüntü kalitesi.
  - `[2] Sadece Ses:` Videodan ayrılmış yüksek kaliteli MP3 ses dosyası.
- **Sistem Güncelleme:** Ayarlar menüsünden tek tıkla `yt-dlp -U` çalıştırma ve motoru güncelleme.
- **Hedef Klasör Seçimi:** Varsayılan olarak Masaüstü seçilidir; Ayarlar üzerinden hedef klasör kolayca değiştirilebilir.
- **Merkezi Başlatma:** Uygulama açıldığında ekran çözünürlüğüne göre otomatik olarak monitörün tam merkezinde açılır.

## Gereksinimler

Programın çalışabilmesi için `VideoIndirici.exe` ile aynı dizinde aşağıdaki bileşenlerin bulunması önerilir:
- `yt-dlp.exe`
- `ffmpeg.exe` & `ffprobe.exe`

## Derleme (Kaynak Koddan)

Rust ve MSVC ortamınız kuruluysa projeyi derlemek için:

```cmd
cd gui
cargo build --release
```

veya kök dizindeki `derle.bat` dosyasını çalıştırabilirsiniz.
