// Modal management module
export class Modal {
    constructor() {
        this.activeModal = null;
        this.init();
    }

    init() {
        // Close modal on click outside
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('modal')) {
                this.close();
            }
        });

        // Close modal on ESC key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && this.activeModal) {
                this.close();
            }
        });

        // Setup close buttons
        document.querySelectorAll('.modal-close').forEach(btn => {
            btn.addEventListener('click', () => this.close());
        });
    }

    open(modalId) {
        const modal = document.getElementById(modalId);
        if (!modal) {
            console.error(`Modal with id ${modalId} not found`);
            return;
        }

        // Close any active modal
        if (this.activeModal) {
            this.close();
        }

        modal.classList.add('active');
        this.activeModal = modal;
        document.body.style.overflow = 'hidden';
    }

    close() {
        if (!this.activeModal) return;

        this.activeModal.classList.remove('active');
        
        // Reset form if present
        const form = this.activeModal.querySelector('form');
        if (form) {
            form.reset();
        }

        this.activeModal = null;
        document.body.style.overflow = '';
    }

    isOpen() {
        return this.activeModal !== null;
    }

    getActiveModal() {
        return this.activeModal;
    }
}

// Export singleton instance
export const modal = new Modal();