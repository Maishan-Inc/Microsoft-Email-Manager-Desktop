/**
 * 发件人品牌 LOGO：按域名匹配本地图标（全部本地，不发任何远程请求，保护隐私）。
 * 图标取自项目 src/assets/brand-icons，规则参考 Microsoft-Email-Manager 网页版。
 * 未命中时返回 null，由界面回退到首字母头像。
 */

// Vite 在构建期把图标打包并给出最终 URL，键为文件名（小写）。
const modules = import.meta.glob("../assets/brand-icons/*.{svg,png}", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;

const assets: Record<string, string> = {};
for (const [path, url] of Object.entries(modules)) {
  const name = path.split("/").pop()!.toLowerCase();
  assets[name] = url;
}

interface LogoRule {
  keywords: string[];
  asset: string;
}

const DOMAIN_LOGO_RULES: LogoRule[] = [
  { keywords: ["microsoft.com", "office.com", "outlook.com", "live.com", "hotmail.com", "microsoftonline.com"], asset: "microsoft-color.svg" },
  { keywords: ["azure.com", "azure.net", "azureedge.net"], asset: "azure-color.svg" },
  { keywords: ["apple.com", "icloud.com", "me.com", "mac.com"], asset: "apple.svg" },
  { keywords: ["google.com", "gmail.com", "googlemail.com"], asset: "google-color.svg" },
  { keywords: ["gemini"], asset: "gemini.svg" },
  { keywords: ["gemma"], asset: "gemma-color.svg" },
  { keywords: ["googlecloud.com", "gcp.com"], asset: "googlecloud-color.svg" },
  { keywords: ["openai.com", "chatgpt.com"], asset: "openai.svg" },
  { keywords: ["anthropic.com"], asset: "anthropic.svg" },
  { keywords: ["claude.ai"], asset: "claude.svg" },
  { keywords: ["ai21.com", "ai21labs.com"], asset: "ai21.png" },
  { keywords: ["cohere.ai", "cohere.com"], asset: "aya.png" },
  { keywords: ["akash.chat", "akash.network"], asset: "akashchat-color.svg" },
  { keywords: ["x.ai", "grok.com"], asset: "xai.svg" },
  { keywords: ["groq.com"], asset: "groq.svg" },
  { keywords: ["mistral.ai"], asset: "mistral.svg" },
  { keywords: ["perplexity.ai", "perplexity.com"], asset: "perplexity-color.svg" },
  { keywords: ["deepseek.com", "deepseek.ai"], asset: "deepseek.svg" },
  { keywords: ["blackforestlabs.ai", "bfl.ai", "flux"], asset: "flux.svg" },
  { keywords: ["moonshot.cn", "moonshot.ai"], asset: "moonshot.svg" },
  { keywords: ["kimi.com"], asset: "kimi.svg" },
  { keywords: ["minimax.chat", "minimax.com"], asset: "minimax.svg" },
  { keywords: ["zhipu.ai", "bigmodel.cn"], asset: "zhipu.svg" },
  { keywords: ["qwen.ai", "tongyi.com", "dashscope.com"], asset: "qwen.svg" },
  { keywords: ["alibaba.com", "alibaba-inc.com", "1688.com"], asset: "alibaba.svg" },
  { keywords: ["alibabacloud.com", "aliyun.com", "aliyuncs.com"], asset: "alibabacloud-color.svg" },
  { keywords: ["aws.amazon.com", "amazon.com", "amazonaws.com"], asset: "aws-color.svg" },
  { keywords: ["cloudflare.com"], asset: "cloudflare-color.svg" },
  { keywords: ["burncloud.com"], asset: "burncloud-color.svg" },
  { keywords: ["tencent.com", "qq.com", "weixin.com", "wechat.com"], asset: "tencent-color.svg" },
  { keywords: ["tencentcloud.com", "tencentcloudapi.com"], asset: "tencentcloud-color.svg" },
  { keywords: ["hunyuan.tencent.com"], asset: "hunyuan.svg" },
  { keywords: ["bytedance.com", "tiktok.com", "douyin.com"], asset: "bytedance.svg" },
  { keywords: ["doubao.com", "doubao.cn"], asset: "doubao-color.svg" },
  { keywords: ["volcengine.com"], asset: "volcengine-color.svg" },
  { keywords: ["baidu.com"], asset: "baidu-brand-color.svg" },
  { keywords: ["ernie.bot", "yiyan.baidu.com", "qianfan"], asset: "ernie.svg" },
  { keywords: ["baiducloud.com", "bcebos.com", "baidubce.com"], asset: "baiducloud-color.svg" },
  { keywords: ["huawei.com", "huaweicloud.com"], asset: "huaweicloud-color.svg" },
  { keywords: ["klingai.com", "kling.ai"], asset: "kling-color.svg" },
  { keywords: ["kolors"], asset: "kolors-color.svg" },
  { keywords: ["kwaipilot.com", "kwai.com", "kuaishou.com"], asset: "kwaipilot.svg" },
  { keywords: ["lumalabs.ai", "luma.ai"], asset: "luma-color.svg" },
  { keywords: ["meta.com", "facebook.com", "instagram.com", "whatsapp.com"], asset: "meta.png" },
  { keywords: ["github.com"], asset: "github.svg" },
  { keywords: ["huggingface.co", "huggingface.com"], asset: "huggingface-color.svg" },
  { keywords: ["modelscope.cn", "modelscope.com"], asset: "modelscope-color.svg" },
  { keywords: ["nvidia.com"], asset: "nvidia-color.svg" },
  { keywords: ["runwayml.com", "runwayml.ai"], asset: "runway.svg" },
  { keywords: ["xfyun.cn", "iflytek.com", "spark"], asset: "spark-color.svg" },
  { keywords: ["statecloud"], asset: "statecloud-color.svg" },
  { keywords: ["suno.ai", "suno.com"], asset: "suno.svg" },
  { keywords: ["upstage.ai"], asset: "upstage-color.svg" },
  { keywords: ["vercel.com"], asset: "vercel.svg" },
  { keywords: ["xiaomi.com"], asset: "xiaomi.svg" },
  { keywords: ["01.ai", "lingyiwanwu.com", "yi.ai"], asset: "yi-color.svg" },
  { keywords: ["commanda.ai", "commanda.com"], asset: "commanda.png" },
  { keywords: ["zeabur.com"], asset: "zeabur-color.png" },
  { keywords: ["railway.app", "railway.com"], asset: "railway.png" },
  { keywords: ["claw.cloud"], asset: "logo-clawcloud.png" },
];

/** 取发件人邮箱地址中 `<a@b.com>` 的纯地址。 */
export function senderAddress(from: string): string {
  const m = from.match(/<([^>]+)>/);
  return (m ? m[1] : from).trim();
}

/** 返回该发件人的品牌 LOGO 资源 URL；未命中返回 null。 */
export function senderLogo(from: string): string | null {
  const addr = senderAddress(from).toLowerCase();
  const domain = addr.includes("@") ? addr.split("@").pop()! : addr;
  if (!domain) return null;
  const rule = DOMAIN_LOGO_RULES.find((r) => r.keywords.some((k) => domain.includes(k)));
  if (!rule) return null;
  return assets[rule.asset.toLowerCase()] ?? null;
}
