#!/usr/bin/env python3
"""
OPENAKTA Backend Test Script

This script tests the OPENAKTA gRPC backend without needing the frontend.

Usage:
    python tests/test_backend.py

Requirements:
    pip install grpcio grpcio-tools
    
Generate protobuf stubs:
    python -m grpc_tools.protoc -I../proto --python_out=. --pyi_out=. --grpc_python_out=. ../proto/*.proto
"""

import asyncio
import grpc
from typing import Optional

# Import generated protobuf stubs (you'll need to generate these)
# import collective_pb2
# import collective_pb2_grpc


class OPENAKTABackendTester:
    """Test the OPENAKTA gRPC backend."""
    
    def __init__(self, address: str = "localhost:50051"):
        self.address = address
        self.channel: Optional[grpc.aio.Channel] = None
        self.stub = None
    
    async def connect(self):
        """Connect to the backend."""
        print(f"🔌 Connecting to {self.address}...")
        self.channel = grpc.aio.insecure_channel(self.address)
        # self.stub = collective_pb2_grpc.CollectiveServiceStub(self.channel)
        print("✅ Connected!")
    
    async def disconnect(self):
        """Disconnect from the backend."""
        if self.channel:
            await self.channel.close()
        print("🔌 Disconnected")
    
    async def test_list_agents(self):
        """Test listing agents."""
        print("\n📋 Testing ListAgents...")
        try:
            # response = await self.stub.ListAgents(
            #     collective_pb2.ListAgentsRequest()
            # )
            # print(f"✅ Found {len(response.agents)} agents")
            # return response
            print("⚠️  Protobuf stubs not generated yet")
            return None
        except grpc.RpcError as e:
            print(f"❌ Error: {e.code()} - {e.details()}")
            return None
    
    async def test_submit_task(self):
        """Test submitting a task."""
        print("\n📝 Testing SubmitTask...")
        try:
            # response = await self.stub.SubmitTask(
            #     collective_pb2.SubmitTaskRequest(
            #         title="Test Task",
            #         description="Testing from Python script",
            #         assignee_id="agent-1"
            #     )
            # )
            # print(f"✅ Task submitted: {response.task.id}")
            # return response
            print("⚠️  Protobuf stubs not generated yet")
            return None
        except grpc.RpcError as e:
            print(f"❌ Error: {e.code()} - {e.details()}")
            return None
    
    async def test_stream_messages(self):
        """Test streaming messages."""
        print("\n💬 Testing StreamMessages...")
        try:
            # request = collective_pb2.StreamMessagesRequest(agent_id="agent-1")
            # response_stream = await self.stub.StreamMessages(request)
            # async for message in response_stream:
            #     print(f"📨 Received: {message.content}")
            # return response_stream
            print("⚠️  Protobuf stubs not generated yet")
            return None
        except grpc.RpcError as e:
            print(f"❌ Error: {e.code()} - {e.details()}")
            return None
    
    async def run_all_tests(self):
        """Run all tests."""
        print("🚀 OPENAKTA Backend Tester")
        print("=" * 50)
        
        await self.connect()
        
        try:
            await self.test_list_agents()
            await self.test_submit_task()
            await self.test_stream_messages()
        finally:
            await self.disconnect()
        
        print("\n" + "=" * 50)
        print("✅ All tests completed!")


async def main():
    """Main entry point."""
    tester = OPENAKTABackendTester()
    await tester.run_all_tests()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Test interrupted by user")
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        exit(1)
