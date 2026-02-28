package node

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func mockHealthServer(freeSlots int) *httptest.Server {
	mux := http.NewServeMux()
	mux.HandleFunc("/api/health", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(&NodeHealth{
			Status:    "ok",
			MaxVMs:    10,
			Running:   10 - freeSlots,
			Paused:    0,
			Total:     10 - freeSlots,
			FreeSlots: freeSlots,
		})
	})
	return httptest.NewServer(mux)
}

func TestScheduler_RegisterAndGetNode(t *testing.T) {
	s := NewScheduler()

	s.RegisterNode(&NodeInfo{
		ID:      "n1",
		BaseURL: "http://n1",
		Secret:  "s1",
		Region:  "us-east",
		IP:      "1.1.1.1",
	})

	n, ok := s.GetNode("n1")
	if !ok {
		t.Fatal("node not found")
	}
	if n.Region != "us-east" {
		t.Errorf("Region = %q, want %q", n.Region, "us-east")
	}

	_, ok = s.GetNode("nonexistent")
	if ok {
		t.Error("expected node not found")
	}
}

func TestScheduler_RemoveNode(t *testing.T) {
	s := NewScheduler()
	s.RegisterNode(&NodeInfo{ID: "n1", BaseURL: "http://n1"})

	s.RemoveNode("n1")

	_, ok := s.GetNode("n1")
	if ok {
		t.Error("expected node to be removed")
	}
}

func TestScheduler_Nodes(t *testing.T) {
	s := NewScheduler()
	s.RegisterNode(&NodeInfo{ID: "n1", BaseURL: "http://n1"})
	s.RegisterNode(&NodeInfo{ID: "n2", BaseURL: "http://n2"})

	nodes := s.Nodes()
	if len(nodes) != 2 {
		t.Errorf("got %d nodes, want 2", len(nodes))
	}
}

func TestScheduler_SelectNode_BestCapacity(t *testing.T) {
	s1 := mockHealthServer(3)
	defer s1.Close()
	s2 := mockHealthServer(7)
	defer s2.Close()

	scheduler := NewScheduler()
	scheduler.RegisterNode(&NodeInfo{ID: "n1", BaseURL: s1.URL, IP: "1.1.1.1"})
	scheduler.RegisterNode(&NodeInfo{ID: "n2", BaseURL: s2.URL, IP: "2.2.2.2"})

	node, health, err := scheduler.SelectNode(context.Background())
	if err != nil {
		t.Fatalf("SelectNode: %v", err)
	}
	if node.ID != "n2" {
		t.Errorf("selected node %q, want %q (more capacity)", node.ID, "n2")
	}
	if health.FreeSlots != 7 {
		t.Errorf("FreeSlots = %d, want 7", health.FreeSlots)
	}
}

func TestScheduler_SelectNode_NoNodes(t *testing.T) {
	scheduler := NewScheduler()
	_, _, err := scheduler.SelectNode(context.Background())
	if err == nil {
		t.Fatal("expected error when no nodes registered")
	}
}

func TestScheduler_SelectNode_AllAtCapacity(t *testing.T) {
	s := mockHealthServer(0)
	defer s.Close()

	scheduler := NewScheduler()
	scheduler.RegisterNode(&NodeInfo{ID: "n1", BaseURL: s.URL, IP: "1.1.1.1"})

	_, _, err := scheduler.SelectNode(context.Background())
	if err == nil {
		t.Fatal("expected error when all nodes at capacity")
	}
}

func TestScheduler_ClientForNode(t *testing.T) {
	scheduler := NewScheduler()
	scheduler.RegisterNode(&NodeInfo{
		ID:      "n1",
		BaseURL: "http://n1:9090",
		Secret:  "secret1",
	})

	client, err := scheduler.ClientForNode("n1")
	if err != nil {
		t.Fatalf("ClientForNode: %v", err)
	}
	if client == nil {
		t.Fatal("expected non-nil client")
	}

	_, err = scheduler.ClientForNode("nonexistent")
	if err == nil {
		t.Fatal("expected error for nonexistent node")
	}
}
