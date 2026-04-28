import { ChatApp } from "@/components/ChatApp";
import { Providers } from "@/components/Providers";

export default function Home() {
  return (
    <Providers>
      <ChatApp />
    </Providers>
  );
}
