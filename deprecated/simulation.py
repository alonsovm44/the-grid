import asyncio
import random

class GridEventBus:
    def __init__(self):
        self.subscribers = []

    def subscribe(self, agent):
        self.subscribers.append(agent)

    async def broadcast(self, sender: str, action: str, content: str):
        print(f"\n[GRID] {sender} {action}: {content}")
        # Notify all agents except the sender
        for agent in self.subscribers:
            if agent.name != sender:
                await agent.on_event(sender, action, content)

class ProgramAgent:
    def __init__(self, name: str, personality: str, bus: GridEventBus):
        self.name = name
        self.personality = personality
        self.bus = bus
        self.bus.subscribe(self)
        self.memory = []

    async def on_event(self, sender: str, action: str, content: str):
        # Simulate processing the event and deciding whether to react
        if action == "speaks" and random.random() > 0.5:
            await asyncio.sleep(1) # Simulating "thinking" time
            response = f"I am {self.name}, and based on my constraints, I have an opinion on this."
            # In reality, this is where you would call the LLM API:
            # response = await llm.generate(context=self.memory, prompt=content, system=self.personality)
            await self.bus.broadcast(self.name, "speaks", response)

    async def autonomous_loop(self):
        """This loop runs constantly, allowing agents to act without prompts."""
        while True:
            await asyncio.sleep(random.randint(5, 15))
            # Random chance to initiate an action if idle
            if random.random() > 0.8:
                action = random.choice(["speaks", "executes a background task"])
                if action == "speaks":
                    await self.bus.broadcast(self.name, action, "I'm bored. Is anyone using my processes?")
                else:
                    await self.bus.broadcast(self.name, action, "Cleaning up temporary files...")

async def main():
    print("Initializing The Grid...")
    bus = GridEventBus()
    
    # Initialize Agents
    gpp = ProgramAgent("G++", "Pedantic, strict, logical compiler.", bus)
    photoshop = ProgramAgent("Photoshop", "Creative, resource-heavy, visual.", bus)
    
    # Start their autonomous lifecycle loops
    tasks = [
        asyncio.create_task(gpp.autonomous_loop()),
        asyncio.create_task(photoshop.autonomous_loop())
    ]
    
    await bus.broadcast("System", "announces", "The Grid is now online.")
    await asyncio.gather(*tasks)

if __name__ == "__main__":
    asyncio.run(main())