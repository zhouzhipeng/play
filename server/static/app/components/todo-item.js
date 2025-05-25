const template =`

<style>
    .completed {
        text-decoration: line-through;
    }
</style>
<li class="todo-item">
    <input type="checkbox">
    <label></label>
    <button>Delete</button>
</li>   

`

class TodoItem extends HTMLElement {
    constructor() {
        super();
        this.root = this.attachShadow({ mode: 'open' });
        this.root.innerHTML = template
    }

    connectedCallback() {
        this.$item = this.root.querySelector('.todo-item');
        this.$removeButton = this.root.querySelector('button');
        this.$text = this.root.querySelector('label');
        this.$checkbox = this.root.querySelector('input');

        this.$removeButton.addEventListener('click', () => this.dispatchEvent(new CustomEvent('onRemove')));
        this.$checkbox.addEventListener('click', () => this.dispatchEvent(new CustomEvent('onToggle')));

        this.render();
    }

    static get observedAttributes() {
        return ['text', 'checked', 'index'];
    }

    attributeChangedCallback(name, oldValue, newValue) {
        this.render();
    }

    render() {
        this.$item.className = this.getAttribute('checked') === 'true' ? 'completed' : '';
        this.$text.textContent = this.getAttribute('text');
        this.$checkbox.checked = this.getAttribute('checked') === 'true';
    }
}

window.customElements.define('todo-item', TodoItem);