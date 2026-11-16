# 本程序是测试代码,作用是在macOS 27系统上,在每次退出视频屏保后,将下一次的视频屏保换成另一个.
## 要求:
1. 在 **~/Library/Application Support/com.apple.wallpaper/aerials/videos/mov** 里存放多个 mov 文件.
2. 将普通视频转换为可以识别的mov文件
   
   假设有一个视频文件 abc.mp4. 鼠标右键, 选择 **"服务"** , **"编码所选视频文件"**, 在弹出的窗口中, 将 **“设置”: HEVC 2160P**.
   
   视频屏保需要 HEVC 格式, 2160P 是分辨率. **点击“继续”**. 稍等一会儿会得到一个同名的mov文件. 将此文件拷贝到 videos 目录.
